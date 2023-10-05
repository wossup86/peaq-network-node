use frame_support::Parameter;
use frame_support::{
	pallet_prelude::{DispatchError, DispatchResult},
	traits::{fungibles},
	traits::{Currency, ExistenceRequirement, Get, Imbalance, OnUnbalanced, WithdrawReasons},
};
use zenlink_protocol::GenerateLpAssetId;
use frame_support::ensure;
use orml_traits::BasicCurrency;
use frame_support::traits::tokens::WithdrawConsequence;
use frame_support::pallet_prelude::InvalidTransaction;
use frame_support::pallet_prelude::TransactionValidityError;
use frame_support::pallet_prelude::MaxEncodedLen;
use frame_support::pallet_prelude::MaybeSerializeDeserialize;
use sp_runtime::traits::CheckedSub;
use orml_traits::MultiCurrency;
use pallet_transaction_payment::{Config as TransPayConfig, OnChargeTransaction};
use pallet_assets::{Config as AssetsConfig};
use frame_system::Config as SysConfig;
use sp_runtime::{
	traits::{
		Convert, DispatchInfoOf, MaybeDisplay, Member, PostDispatchInfoOf, SaturatedConversion,
		Saturating, Zero,
	},
	Perbill, RuntimeString,
};
use sp_std::{fmt::Debug, marker::PhantomData, vec, vec::Vec};
use crate::EoTFeeFactor;

use peaq_primitives_xcm::{
	NewZenlinkAssetId,
	PeaqAssetId,
	Amount,
};
use peaq_primitives_xcm::NewCurrencyId;
use zenlink_protocol::{
	AssetBalance, AssetId as ZenlinkAssetId, Config as ZenProtConfig, ExportZenlink,
	LocalAssetHandler,
};
use frame_support::{
	traits::{
		Currency as PalletCurrency
	}
};


use crate::{log, log_internal, log_icon};

pub struct PeaqMultiCurrenciesWrapper<T, MultiCurrencies, NativeCurrency, GetNativeCurrencyId>
	(PhantomData<(T, MultiCurrencies, NativeCurrency, GetNativeCurrencyId)>);

impl<T, MultiCurrencies, NativeCurrency, GetNativeCurrencyId> MultiCurrency<T::AccountId>
	for PeaqMultiCurrenciesWrapper<T, MultiCurrencies, NativeCurrency, GetNativeCurrencyId>
where
	MultiCurrencies: fungibles::Mutate<T::AccountId>
		+ fungibles::Inspect<T::AccountId, AssetId = T::AssetId, Balance = T::Balance>
		+ fungibles::Transfer<T::AccountId>,
	NativeCurrency: BasicCurrency<T::AccountId, Balance = T::Balance>,
	GetNativeCurrencyId: Get<T::AssetId>,
	T: SysConfig + AssetsConfig,
{
	type CurrencyId = T::AssetId;
	type Balance = T::Balance;

	fn minimum_balance(currency_id: Self::CurrencyId) -> Self::Balance {
		if currency_id == GetNativeCurrencyId::get() {
			let out = NativeCurrency::minimum_balance();
			log::error!("NativeCurrency::minimum_balance: out: {:?}", out);
			out
		} else {
			let out = MultiCurrencies::minimum_balance(currency_id);
			log::error!("MultiCurrencies::minimum_balance: out: {:?}", out);
			out
		}
	}

	fn total_issuance(currency_id: Self::CurrencyId) -> Self::Balance {
		if currency_id == GetNativeCurrencyId::get() {
			let out = NativeCurrency::total_issuance();
			log::error!("NativeCurrency::total_issuance: out: {:?}", out);
			out
		} else {
			let out = MultiCurrencies::total_issuance(currency_id);
			log::error!("MultiCurrencies::total_issuance: out: {:?}", out);
			out
		}
	}

	fn total_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		if currency_id == GetNativeCurrencyId::get() {
			let out = NativeCurrency::total_balance(who);
			log::error!("NativeCurrency::total_balance: out: {:?}", out);
			out
		} else {
			let out = MultiCurrencies::balance(currency_id, who);
			log::error!("MultiCurrencies::balance: out: {:?}", out);
			out
		}
	}

	fn free_balance(currency_id: Self::CurrencyId, who: &T::AccountId) -> Self::Balance {
		if currency_id == GetNativeCurrencyId::get() {
			let out = NativeCurrency::free_balance(who);
			log::error!("NativeCurrency::free_balance: out: {:?}", out);
			out
		} else {
			let out = MultiCurrencies::balance(currency_id, who);
			log::error!("MultiCurrencies::balance: out: {:?}", out);
			out
		}
	}

	fn ensure_can_withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if currency_id == GetNativeCurrencyId::get() {
			log::error!(
				"Show: ensure_can_withdraw: currency_id: {:?}, who: {:?}, amount: {:?}",
				currency_id,
				who,
				amount
			);

			let aa = NativeCurrency::ensure_can_withdraw(who, amount);
			log::error!("Show: ensure_can_withdraw: aa: {:?}", aa);
			return aa
		} else {
			log::error!(
				"Show: ensure_can_withdraw: currency_id: {:?}, who: {:?}, amount: {:?}",
				currency_id,
				who,
				amount
			);
			let out = MultiCurrencies::can_withdraw(currency_id, who, amount);
			log::error!("Show: ensure_can_withdraw: out: {:?}", out);
			if WithdrawConsequence::Success == out {
				return Ok(())
			} else {
				return Err(DispatchError::Other("Insufficient balance"))
			}
		}
	}

	fn transfer(
		currency_id: Self::CurrencyId,
		from: &T::AccountId,
		to: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		if amount.is_zero() || from == to {
			return Ok(());
		}
		if currency_id == GetNativeCurrencyId::get() {
			let out = NativeCurrency::transfer(from, to, amount);
			log::error!("NativeCurrency::transfer: out: {:?}", out);
			out
		} else {
			// TODO...
			let out = MultiCurrencies::transfer(currency_id, from, to, amount, true);
			log::error!("MultiCurrencies::transfer: out: {:?}", out);
			if out.is_ok() {
				return Ok(())
			} else {
				return Err(DispatchError::Other("Transfer failed"))
			}
		}
	}

	fn deposit(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		if currency_id == GetNativeCurrencyId::get() {
			let out = NativeCurrency::deposit(who, amount);
			log::error!("NativeCurrency::deposit: out: {:?}", out);
			out
		} else {
			let out = MultiCurrencies::mint_into(currency_id, who, amount);
			log::error!("MultiCurrencies::mint_into: out: {:?}", out);
			if out.is_ok() {
				return Ok(())
			} else {
				log::error!("Show: deposit: out: {:?}", out);
				return Err(DispatchError::Other("Deposit failed"))
			}
		}
	}

	fn withdraw(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		if amount.is_zero() {
			return Ok(());
		}
		if currency_id == GetNativeCurrencyId::get() {
			let out = NativeCurrency::withdraw(who, amount);
			log::error!("NativeCurrency::withdraw: out: {:?}", out);
			out
		} else {
			let out = MultiCurrencies::burn_from(currency_id, who, amount);
			log::error!("MultiCurrencies::burn_from: out: {:?}", out);
			if out.is_ok() {
				return Ok(())
			} else {
				log::error!("MultiCurrencies::transfer: out: {:?}", out);
				return Err(DispatchError::Other("Withdraw failed"))
			}
		}
	}

	fn can_slash(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> bool {
		if currency_id == GetNativeCurrencyId::get() {
			let out = NativeCurrency::can_slash(who, amount);
			log::error!("NativeCurrency::can_slash: out: {:?}", out);
			out
		} else {
			let out = Self::free_balance(currency_id, who) >= amount;
			log::error!("Self::can_slash: out: {:?}", out);
			out
		}
	}

	fn slash(currency_id: Self::CurrencyId, who: &T::AccountId, amount: Self::Balance) -> Self::Balance {
		if currency_id == GetNativeCurrencyId::get() {
			let out = NativeCurrency::slash(who, amount);
			log::error!("NativeCurrency::slash: out: {:?}", out);
			out
		} else {
			let out = MultiCurrencies::slash(currency_id, who, amount).ok().unwrap();
			log::error!("MultiCurrencies::slash: out: {:?}", out);
			out
		}
	}
}

/// A local adaptor to convert between Zenlink-Assets and Peaq's local currency.
pub struct PeaqNewLocalAssetAdaptor<Local>(PhantomData<Local>);

impl<Local, AccountId> LocalAssetHandler<AccountId> for PeaqNewLocalAssetAdaptor<Local>
where
	Local: MultiCurrency<AccountId, CurrencyId = NewCurrencyId>,
	AccountId: Debug,
{
	fn local_balance_of(asset_id: NewZenlinkAssetId, who: &AccountId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			log::error!("PeaqNewLocalAssetAdaptor: local_balance_of: currency_id: {:?}, who: {:?}", currency_id, who);
			let aa = TryInto::<AssetBalance>::try_into(Local::free_balance(currency_id, who))
				.unwrap_or_default();
			log::error!("PeaqNewLocalAssetAdaptor: local_balance_of: aa: {:?}", aa);
			return aa;
		}
		log::error!("fail PeaqNewLocalAssetAdaptor: local_balance_of: asset_id: {:?}, who: {:?}", asset_id, who);
		AssetBalance::default()
	}

	fn local_total_supply(asset_id: NewZenlinkAssetId) -> AssetBalance {
		if let Ok(currency_id) = asset_id.try_into() {
			log::error!("PeaqNewLocalAssetAdaptor: local_total_supply: currency_id: {:?}", currency_id);
			let aa = TryInto::<AssetBalance>::try_into(Local::total_issuance(currency_id))
				.unwrap_or_default();
			log::error!("PeaqNewLocalAssetAdaptor: local_total_supply: aa: {:?}", aa);
			return aa;
		}
		log::error!("fail PeaqNewLocalAssetAdaptor: local_total_supply: asset_id: {:?}", asset_id);
		AssetBalance::default()
	}

	fn local_is_exists(asset_id: NewZenlinkAssetId) -> bool {
		log::error!("PeaqNewLocalAssetAdaptor: local_is_exists: asset_id: {:?}", asset_id);
		let currency_id: Result<NewCurrencyId, ()> = asset_id.try_into();
		log::error!("PeaqNewLocalAssetAdaptor: local_is_exists: currency_id: {:?}", currency_id);
		currency_id.is_ok()
	}

	fn local_transfer(
		asset_id: NewZenlinkAssetId,
		origin: &AccountId,
		target: &AccountId,
		amount: AssetBalance,
	) -> DispatchResult {
		if let Ok(currency_id) = asset_id.try_into() {
			log::error!("PeaqNewLocalAssetAdaptor: local_transfer: currency_id: {:?}, origin: {:?}, target: {:?}, amount: {:?}", currency_id, origin, target, amount);
			let aa = Local::transfer(
				currency_id,
				origin,
				target,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local transfer"))?,
			);
			log::error!("PeaqNewLocalAssetAdaptor: local_transfer: aa: {:?}", aa);
			return aa;
		} else {
			log::error!("fail PeaqNewLocalAssetAdaptor: local_transfer: asset_id: {:?}, origin: {:?}, target: {:?}, amount: {:?}", asset_id, origin, target, amount);
			Err(DispatchError::Other("unknown asset in local transfer"))
		}
	}

	fn local_deposit(
		asset_id: NewZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(currency_id) = asset_id.try_into() {
			log::error!("PeaqNewLocalAssetAdaptor: local_deposit: currency_id: {:?}, origin: {:?}, amount: {:?}", currency_id, origin, amount);
			let aa = Local::deposit(
				currency_id,
				origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local deposit"))?,
			);
			log::error!("PeaqNewLocalAssetAdaptor: local_deposit: aa: {:?}", aa);
			aa?
		} else {
			log::error!("fail PeaqNewLocalAssetAdaptor: local_deposit: asset_id: {:?}, origin: {:?}, amount: {:?}", asset_id, origin, amount);
			return Err(DispatchError::Other("unknown asset in local transfer"))
		}

		Ok(amount)
	}

	fn local_withdraw(
		asset_id: NewZenlinkAssetId,
		origin: &AccountId,
		amount: AssetBalance,
	) -> Result<AssetBalance, DispatchError> {
		if let Ok(currency_id) = asset_id.try_into() {
			log::error!("PeaqNewLocalAssetAdaptor: local_withdraw: currency_id: {:?}, origin: {:?}, amount: {:?}", currency_id, origin, amount);
			let aa = Local::withdraw(
				currency_id,
				origin,
				amount
					.try_into()
					.map_err(|_| DispatchError::Other("convert amount in local withdraw"))?,
			);
			log::error!("PeaqNewLocalAssetAdaptor: local_withdraw: aa: {:?}", aa);
			aa?;
		} else {
			return Err(DispatchError::Other("unknown asset in local transfer"))
		}

		Ok(amount)
	}
}

type BalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::Balance;
type BalanceOfA<C, A> = <C as Currency<A>>::Balance;
type NegativeImbalanceOf<C, T> = <C as Currency<<T as SysConfig>::AccountId>>::NegativeImbalance;

/// Simple encapsulation of multiple return values.
#[derive(Debug)]
pub struct NewPaymentConvertInfo {
	/// Needed amount-in for token swap.
	pub amount_in: AssetBalance,
	/// Resulting amount-out after token swap.
	pub amount_out: AssetBalance,
	/// Zenlink's path of token-pair.
	pub zen_path: Vec<NewZenlinkAssetId>,
}


// [TODO] Need to modify
/// Peaq's Currency Adapter to apply EoT-Fee and to enable withdrawal from foreign currencies.
pub struct NewPeaqCurrencyAdapter<C, OU, PCPC>(PhantomData<(C, OU, PCPC)>);

impl<T, C, OU, PCPC> OnChargeTransaction<T> for NewPeaqCurrencyAdapter<C, OU, PCPC>
where
	T: SysConfig + TransPayConfig + ZenProtConfig,
	C: Currency<T::AccountId>,
	OU: OnUnbalanced<NegativeImbalanceOf<C, T>>,
	PCPC: NewPeaqCurrencyPaymentConvert<AccountId = T::AccountId, Currency = C>,
	AssetBalance: From<BalanceOf<C, T>>,
{
	type LiquidityInfo = Option<NegativeImbalanceOf<C, T>>;
	type Balance = <C as Currency<T::AccountId>>::Balance;

	/// Withdraw the predicted fee from the transaction origin.
	/// Note: The `fee` already includes the `tip`.
	fn withdraw_fee(
		who: &T::AccountId,
		_call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		total_fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		if total_fee.is_zero() {
			return Ok(None)
		}
		let inclusion_fee = total_fee - tip;

		let withdraw_reason = if tip.is_zero() {
			WithdrawReasons::TRANSACTION_PAYMENT
		} else {
			WithdrawReasons::TRANSACTION_PAYMENT | WithdrawReasons::TIP
		};

		// Apply Peaq Economy-of-Things Fee adjustment.
		let eot_fee = EoTFeeFactor::get() * inclusion_fee;
		let tx_fee = total_fee.saturating_add(eot_fee);

		// Check if user can withdraw in any valid currency.
		let currency_id = PCPC::ensure_can_withdraw(who, tx_fee)?;
		log::error!("WWW {:?}", currency_id);
		// [TODO].. .That's weird
		if !currency_id.is_native_token() {
			log!(
				info,
				NewPeaqCurrencyAdapter,
				"Payment with swap of {:?}-tokens",
				currency_id
			);
		}

		match C::withdraw(who, tx_fee, withdraw_reason, ExistenceRequirement::KeepAlive) {
			Ok(imbalance) => Ok(Some(imbalance)),
			Err(_) => Err(InvalidTransaction::Payment.into()),
		}
	}

	/// Hand the fee and the tip over to the `[OnUnbalanced]` implementation.
	/// Since the predicted fee might have been too high, parts of the fee may
	/// be refunded.
	/// Note: The `corrected_fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &T::AccountId,
		_dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		_post_info: &PostDispatchInfoOf<T::RuntimeCall>,
		cor_total_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		if let Some(paid) = already_withdrawn {
			// Apply same Peaq Economy-of-Things Fee adjustment as above
			let cor_inclusion_fee = cor_total_fee - tip;
			let cor_eot_fee = EoTFeeFactor::get() * cor_inclusion_fee;
			let cor_tx_fee = cor_total_fee.saturating_add(cor_eot_fee);

			// Calculate how much refund we should return
			let refund_amount = paid.peek().saturating_sub(cor_tx_fee);
			// refund to the the account that paid the fees. If this fails, the
			// account might have dropped below the existential balance. In
			// that case we don't refund anything.
			let refund_imbalance = C::deposit_into_existing(who, refund_amount)
				.unwrap_or_else(|_| C::PositiveImbalance::zero());
			// merge the imbalance caused by paying the fees and refunding parts of it again.
			let adjusted_paid = paid
				.offset(refund_imbalance)
				.same()
				.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;
			// Call someone else to handle the imbalance (fee and tip separately)
			let (tip, fee) = adjusted_paid.split(tip);

			OU::on_unbalanceds(Some(fee).into_iter().chain(Some(tip)));
		}
		Ok(())
	}
}

/// Individual trait to handle payments in non-local currencies. The intention is to keep it as
/// generic as possible to enable the usage in PeaqCurrencyAdapter.
pub trait NewPeaqCurrencyPaymentConvert {
	/// AccountId type.
	type AccountId: Parameter
		+ Member
		+ MaybeSerializeDeserialize
		+ Debug
		+ MaybeDisplay
		+ Ord
		+ MaxEncodedLen;

	/// Currency type.
	type Currency: Currency<Self::AccountId>;

	/// MultiCurrency, should be orml-currencies.
	type MultiCurrency: MultiCurrency<
		Self::AccountId,
		CurrencyId = NewCurrencyId,
		Balance = BalanceOfA<Self::Currency, Self::AccountId>,
	>;

	/// Zenlink-DEX-Protocol.
	type DexOperator: ExportZenlink<Self::AccountId, ZenlinkAssetId>;

	/// Existential deposit.
	type ExistentialDeposit: Get<BalanceOfA<Self::Currency, Self::AccountId>>;

	/// Local NewCurrencyId in type of Zenlink's AssetId.
	type NativeCurrencyId: Get<NewCurrencyId>;

	/// List of all accepted CurrencyIDs except for the local ones in type of Zenlink's AssetId.
	type LocalAcceptedIds: Get<Vec<NewCurrencyId>>;

	/// This method checks if the fee can be withdrawn in any currency and returns the asset_id
	/// of the choosen currency in dependency of the priority-list and availability of tokens.
	fn ensure_can_withdraw(
		who: &Self::AccountId,
		tx_fee: BalanceOfA<Self::Currency, Self::AccountId>,
	) -> Result<NewCurrencyId, TransactionValidityError> {
		let (currency_id, option) = Self::check_currencies_n_priorities(who, tx_fee)?;

		if let Some(info) = option {
			Self::DexOperator::inner_swap_assets_for_exact_assets(
				who,
				info.amount_out,
				info.amount_in,
				&info.zen_path,
				who,
			)
			.map_err(|_| map_err_newcurrency2zasset(currency_id))?;
		}
		log::error!(
			"QQ Show: ensure_can_withdraw: currency_id: {:?}",
			currency_id
		);
		Ok(currency_id)
	}

	/// Checks all accepted native currencies and selects the first with enough tokens.
	fn check_currencies_n_priorities(
		who: &Self::AccountId,
		tx_fee: BalanceOfA<Self::Currency, Self::AccountId>,
	) -> Result<(NewCurrencyId, Option<NewPaymentConvertInfo>), TransactionValidityError> {
		let native_id = Self::NativeCurrencyId::get();

		log::error!(
			"Show: check_currencies_n_priorities: native_id: {:?}, who: {:?} tx_fee: {:?}",
			native_id,
			who,
			tx_fee
		);
		let qq = Self::MultiCurrency::ensure_can_withdraw(native_id, who, tx_fee);
		log::error!(
			"Show: ensure_can_withdraw: {:?}", qq
		);
		if qq.is_ok() {
			log::error!(
				"Show: check_currencies_n_priorities: native_id: {:?}, who: {:?} tx_fee: {:?}",
				native_id,
				who,
				tx_fee
			);
			Ok((native_id, None))
		} else {
			// In theory not necessary, but as safety-buffer will add existential deposit.
			let tx_fee = tx_fee.saturating_add(Self::ExistentialDeposit::get());

			// Prepare ZenlinkAssetId(s) from NewCurrencyId(s).
			let native_zen_id = ZenlinkAssetId::try_from(native_id)
				.map_err(|_| map_err_newcurrency2zasset(native_id))?;
			let local_ids = Self::LocalAcceptedIds::get();

			// Iterate through all accepted local currencies and check availability.
			for &local_id in local_ids.iter() {
				// TODO
				let local_zen_id = ZenlinkAssetId::try_from(local_id)
					.map_err(|_| map_err_newcurrency2zasset(local_id))?;
				let zen_path = vec![local_zen_id, native_zen_id];
				let amount_out: AssetBalance = tx_fee.saturated_into();

				if let Ok(amounts) = Self::DexOperator::get_amount_in_by_path(amount_out, &zen_path)
				{
					let amount_in =
						BalanceOfA::<Self::Currency, Self::AccountId>::saturated_from(amounts[0]);
					if Self::MultiCurrency::ensure_can_withdraw(local_id, who, amount_in).is_ok() {
						let info =
							NewPaymentConvertInfo { amount_in: amounts[0], amount_out, zen_path };
						return Ok((local_id, Some(info)))
					}
				}
			}
			log::error!(" QQQQQQ");
			Err(InvalidTransaction::Payment.into())
		}
	}
}

fn map_err_newcurrency2zasset(id: NewCurrencyId) -> TransactionValidityError {
	InvalidTransaction::Custom(match id {
		NewCurrencyId::Token(symbol) => symbol as u8,
		_ => 255u8,
	})
	.into()
}


/// Adapt other currency traits implementation to `BasicCurrency`.
pub struct PeaqBasicCurrencyAdapter<Currency>(PhantomData<Currency>);

type PalletBalanceOf<A, Currency> = <Currency as PalletCurrency<A>>::Balance;

// Adapt `frame_support::traits::Currency`
impl<AccountId, Currency> BasicCurrency<AccountId>
	for PeaqBasicCurrencyAdapter<Currency>
where
	Currency: PalletCurrency<AccountId>,
{
	type Balance = PalletBalanceOf<AccountId, Currency>;

	fn minimum_balance() -> Self::Balance {
		Currency::minimum_balance()
	}

	fn total_issuance() -> Self::Balance {
		Currency::total_issuance()
	}

	fn total_balance(who: &AccountId) -> Self::Balance {
		Currency::total_balance(who)
	}

	fn free_balance(who: &AccountId) -> Self::Balance {
		Currency::free_balance(who)
	}

	fn ensure_can_withdraw(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		let new_balance = Self::free_balance(who)
			.checked_sub(&amount)
			.ok_or(DispatchError::Other("Insufficient balance"))?;

		Currency::ensure_can_withdraw(who, amount, WithdrawReasons::all(), new_balance)
	}

	fn transfer(from: &AccountId, to: &AccountId, amount: Self::Balance) -> DispatchResult {
		log::error!("QQ Show: transfer: from: {:?}, to: {:?}, amount: {:?}", from, to, amount);
		Currency::transfer(from, to, amount, ExistenceRequirement::AllowDeath)
	}

	fn deposit(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		if !amount.is_zero() {
			let deposit_result = Currency::deposit_creating(who, amount);
			let actual_deposit = deposit_result.peek();
			ensure!(actual_deposit == amount, DispatchError::Other("Deposit failed"));
		}
		Ok(())
	}

	fn withdraw(who: &AccountId, amount: Self::Balance) -> DispatchResult {
		Currency::withdraw(who, amount, WithdrawReasons::all(), ExistenceRequirement::AllowDeath).map(|_| ())
	}

	fn can_slash(who: &AccountId, amount: Self::Balance) -> bool {
		Currency::can_slash(who, amount)
	}

	fn slash(who: &AccountId, amount: Self::Balance) -> Self::Balance {
		let (_, gap) = Currency::slash(who, amount);
		gap
	}
}

/// This is the Peaq's default GenerateLpAssetId implementation.
pub struct NewPeaqZenlinkLpGenerate<T, Local, ExistentialDeposit, AdminAccount>(PhantomData<(T,
Local, ExistentialDeposit, AdminAccount)>);

impl<T, Local, ExistentialDeposit, AdminAccount> GenerateLpAssetId<NewZenlinkAssetId> for
	NewPeaqZenlinkLpGenerate<T, Local, ExistentialDeposit, AdminAccount>
where
	Local: fungibles::Create<T::AccountId, AssetId = NewCurrencyId, Balance = T::Balance> +
		fungibles::Inspect<T::AccountId, AssetId = NewCurrencyId, Balance = T::Balance>,
	T: SysConfig + AssetsConfig,
	ExistentialDeposit: Get<T::Balance>,
	AdminAccount: Get<T::AccountId>,
{
	fn generate_lp_asset_id(
		asset0: NewZenlinkAssetId,
		asset1: NewZenlinkAssetId,
	) -> Option<NewZenlinkAssetId> {
		let asset_id0: PeaqAssetId = asset0.try_into().ok()?;
		let asset_id1: PeaqAssetId = asset1.try_into().ok()?;

		match (asset_id0, asset_id1) {
			(NewCurrencyId::Token(symbol0), NewCurrencyId::Token(symbol1)) => {
				let lp_currency = NewCurrencyId::LPToken(symbol0, symbol1);
				if !Local::asset_exists(lp_currency) {
					// [TODO] That's so weird if somebody send the rpc and create the lp token...
					// [TODO] Set metadata
					Local::create(lp_currency, AdminAccount::get(), true, ExistentialDeposit::get()).ok()?;
				}
				lp_currency.try_into().ok()
			},
			(_, _) => None,
		}
	}
}
