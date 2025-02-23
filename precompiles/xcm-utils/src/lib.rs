// Copyright 2019-2022 PureStake Inc.
// This file is part of Moonbeam.

// Moonbeam is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Moonbeam is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Moonbeam.  If not, see <http://www.gnu.org/licenses/>.

//! Precompile to xcm utils runtime methods via the EVM

#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use frame_support::{
	dispatch::{GetDispatchInfo, PostDispatchInfo},
	traits::ConstU32,
};
use pallet_evm::AddressMapping;
use parity_scale_codec::{Decode, DecodeLimit, MaxEncodedLen};
use precompile_utils::{precompile_set::SelectorFilter, prelude::*};
use sp_core::{H160, U256};
use sp_runtime::traits::Dispatchable;
use sp_std::{boxed::Box, marker::PhantomData, vec, vec::Vec};
use sp_weights::Weight;
use xcm::{latest::prelude::*, VersionedXcm, MAX_XCM_DECODE_DEPTH};
use xcm_executor::traits::{WeightBounds, WeightTrader};

pub type SystemCallOf<Runtime> = <Runtime as frame_system::Config>::RuntimeCall;
pub const XCM_SIZE_LIMIT: u32 = 2u32.pow(16);
type GetXcmSizeLimit = ConstU32<XCM_SIZE_LIMIT>;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct AllExceptXcmExecute<Runtime, XcmConfig>(PhantomData<(Runtime, XcmConfig)>);

impl<Runtime, XcmConfig> SelectorFilter for AllExceptXcmExecute<Runtime, XcmConfig>
where
	Runtime: pallet_evm::Config + frame_system::Config + pallet_xcm::Config,
	XcmConfig: xcm_executor::Config,
	SystemCallOf<Runtime>: Dispatchable<PostInfo = PostDispatchInfo> + Decode + GetDispatchInfo,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_xcm::Call<Runtime>>,
{
	fn is_allowed(_caller: H160, selector: Option<u32>) -> bool {
		match selector {
			None => true,
			Some(selector) =>
				!XcmUtilsPrecompileCall::<Runtime, XcmConfig>::xcm_execute_selectors()
					.contains(&selector),
		}
	}

	fn description() -> String {
		"Allowed for all callers for all selectors except 'execute'".into()
	}
}

/// A precompile to wrap the functionality from xcm-utils
pub struct XcmUtilsPrecompile<Runtime, XcmConfig>(PhantomData<(Runtime, XcmConfig)>);

#[precompile_utils::precompile]
impl<Runtime, XcmConfig> XcmUtilsPrecompile<Runtime, XcmConfig>
where
	Runtime: pallet_evm::Config + frame_system::Config + pallet_xcm::Config,
	XcmConfig: xcm_executor::Config,
	SystemCallOf<Runtime>: Dispatchable<PostInfo = PostDispatchInfo> + Decode + GetDispatchInfo,
	<<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin:
		From<Option<Runtime::AccountId>>,
	<Runtime as frame_system::Config>::RuntimeCall: From<pallet_xcm::Call<Runtime>>,
{
	// Remove multilocationToAddress

	#[precompile::public("getUnitsPerSecond((uint8,bytes[]))")]
	#[precompile::public("get_units_per_second((uint8,bytes[]))")]
	#[precompile::view]
	fn get_units_per_second(
		handle: &mut impl PrecompileHandle,
		multilocation: MultiLocation,
	) -> EvmResult<U256> {
		// storage item: AssetTypeUnitsPerSecond
		// max encoded len: hash (16) + Multilocation + u128 (16)
		handle.record_db_read::<Runtime>(32 + MultiLocation::max_encoded_len())?;

		// We will construct an asset with the max amount, and check how much we
		// get in return to substract
		let multiasset: xcm::latest::MultiAsset = (multilocation, u128::MAX).into();
		let weight_per_second = 1_000_000_000_000u64;

		let mut trader = <XcmConfig as xcm_executor::Config>::Trader::new();

		// buy_weight returns unused assets
		let unused = trader
			.buy_weight(
				Weight::from_parts(weight_per_second, DEFAULT_PROOF_SIZE),
				vec![multiasset.clone()].into(),
			)
			.map_err(|_| {
				RevertReason::custom("Asset not supported as fee payment").in_field("multilocation")
			})?;

		// we just need to substract from u128::MAX the unused assets
		if let Some(amount) = unused
			.fungible
			.get(&multiasset.id)
			.map(|&value| u128::MAX.saturating_sub(value))
		{
			Ok(amount.into())
		} else {
			Err(revert("Weight was too expensive to be bought with this asset"))
		}
	}

	#[precompile::public("weightMessage(bytes)")]
	#[precompile::public("weight_message(bytes)")]
	#[precompile::view]
	fn weight_message(
		_handle: &mut impl PrecompileHandle,
		message: BoundedBytes<GetXcmSizeLimit>,
	) -> EvmResult<u64> {
		let message: Vec<u8> = message.into();

		let msg =
			VersionedXcm::<<XcmConfig as xcm_executor::Config>::RuntimeCall>::decode_all_with_depth_limit(
				MAX_XCM_DECODE_DEPTH,
				&mut message.as_slice(),
			)
			.map(Xcm::<<XcmConfig as xcm_executor::Config>::RuntimeCall>::try_from);

		let result = match msg {
			Ok(Ok(mut x)) =>
				XcmConfig::Weigher::weight(&mut x).map_err(|_| revert("failed weighting")),
			_ => Err(RevertReason::custom("Failed decoding").in_field("message").into()),
		};

		Ok(result?.ref_time())
	}

	#[precompile::public("xcmExecute(bytes,uint64)")]
	#[precompile::public("xcm_execute(bytes,uint64)")]
	fn xcm_execute(
		handle: &mut impl PrecompileHandle,
		message: BoundedBytes<GetXcmSizeLimit>,
		weight: u64,
	) -> EvmResult {
		let message: Vec<u8> = message.into();

		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

		let message: Vec<_> = message.to_vec();
		let xcm = xcm::VersionedXcm::<SystemCallOf<Runtime>>::decode_all_with_depth_limit(
			xcm::MAX_XCM_DECODE_DEPTH,
			&mut message.as_slice(),
		)
		.map_err(|_e| RevertReason::custom("Failed xcm decoding").in_field("message"))?;

		let call = pallet_xcm::Call::<Runtime>::execute {
			message: Box::new(xcm),
			max_weight: Weight::from_parts(weight, DEFAULT_PROOF_SIZE),
		};

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}

	#[precompile::public("xcmSend((uint8,bytes[]),bytes)")]
	#[precompile::public("xcm_send((uint8,bytes[]),bytes)")]
	fn xcm_send(
		handle: &mut impl PrecompileHandle,
		dest: MultiLocation,
		message: BoundedBytes<GetXcmSizeLimit>,
	) -> EvmResult {
		let message: Vec<u8> = message.into();

		let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

		let message: Vec<_> = message.to_vec();
		let xcm = xcm::VersionedXcm::<()>::decode_all_with_depth_limit(
			xcm::MAX_XCM_DECODE_DEPTH,
			&mut message.as_slice(),
		)
		.map_err(|_e| RevertReason::custom("Failed xcm decoding").in_field("message"))?;

		let call = pallet_xcm::Call::<Runtime>::send {
			dest: Box::new(dest.into()),
			message: Box::new(xcm),
		};

		RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call, 0)?;

		Ok(())
	}
}
