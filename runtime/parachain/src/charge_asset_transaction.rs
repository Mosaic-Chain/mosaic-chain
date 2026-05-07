use sdk::{
	frame_support, pallet_asset_conversion, pallet_asset_conversion_tx_payment, sp_core,
	sp_runtime, staging_xcm,
};

use frame_support::traits::{
	fungibles::{self, Credit},
	tokens::{
		imbalance::ResolveAssetTo, ConversionFromAssetBalance, ConversionToAssetBalance, Fortitude,
		Precision, Preservation,
	},
};
use sp_core::Get;
use sp_runtime::DispatchError;
use staging_xcm::latest::Location;

use pallet_asset_conversion::{QuotePrice, SwapCredit};
use pallet_asset_conversion_tx_payment::SwapAssetAdapter;

use super::{
	configs::parachain::NativeAndAssets, funds, xcm_config, AccountId, AssetConversion, AssetRate,
	Balance, Vec,
};
use funds::treasury::Account as TreasuryAccount;

pub type AssetConverter = AssetConverterChain<
	FundAssetConversion<funds::financial_fund::Account, AssetRate, NativeAndAssets>,
	AssetConversion,
>;

pub type ChargeAssetTransaction = SwapAssetAdapter<
	xcm_config::HereLocation,
	NativeAndAssets,
	AssetConverter,
	ResolveAssetTo<TreasuryAccount, NativeAndAssets>,
>;

pub struct AssetConverterChain<Left, Right> {
	_phantom: core::marker::PhantomData<(Left, Right)>,
}

impl<Left, Right> QuotePrice for AssetConverterChain<Left, Right>
where
	Left: QuotePrice,
	Right: QuotePrice<
		Balance = <Left as QuotePrice>::Balance,
		AssetKind = <Left as QuotePrice>::AssetKind,
	>,
	Left::AssetKind: Clone,
{
	type Balance = Left::Balance;
	type AssetKind = Left::AssetKind;

	fn quote_price_tokens_for_exact_tokens(
		asset1: Self::AssetKind,
		asset2: Self::AssetKind,
		amount: Self::Balance,
		include_fee: bool,
	) -> Option<Self::Balance> {
		Left::quote_price_tokens_for_exact_tokens(
			asset1.clone(),
			asset2.clone(),
			amount,
			include_fee,
		)
		.or_else(|| Right::quote_price_tokens_for_exact_tokens(asset1, asset2, amount, include_fee))
	}

	fn quote_price_exact_tokens_for_tokens(
		asset1: Self::AssetKind,
		asset2: Self::AssetKind,
		amount: Self::Balance,
		include_fee: bool,
	) -> Option<Self::Balance> {
		Left::quote_price_exact_tokens_for_tokens(
			asset1.clone(),
			asset2.clone(),
			amount,
			include_fee,
		)
		.or_else(|| Right::quote_price_exact_tokens_for_tokens(asset1, asset2, amount, include_fee))
	}
}

impl<AccountIdd, Left, Right> SwapCredit<AccountIdd> for AssetConverterChain<Left, Right>
where
	Left: SwapCredit<AccountIdd>,
	Right: SwapCredit<
		AccountIdd,
		Balance = <Left as SwapCredit<AccountIdd>>::Balance,
		AssetKind = <Left as SwapCredit<AccountIdd>>::AssetKind,
		Credit = <Left as SwapCredit<AccountIdd>>::Credit,
	>,
	Left::AssetKind: Clone,
{
	type Balance = <Left as SwapCredit<AccountIdd>>::Balance;
	type AssetKind = <Left as SwapCredit<AccountIdd>>::AssetKind;
	type Credit = <Left as SwapCredit<AccountIdd>>::Credit;

	fn max_path_len() -> u32 {
		Left::max_path_len().min(Right::max_path_len())
	}

	fn swap_exact_tokens_for_tokens(
		path: Vec<Self::AssetKind>,
		credit_in: Self::Credit,
		amount_out_min: Option<Self::Balance>,
	) -> Result<Self::Credit, (Self::Credit, sdk::sp_runtime::DispatchError)> {
		Left::swap_exact_tokens_for_tokens(path.clone(), credit_in, amount_out_min)
			.inspect_err(|e| log::debug!("Failed to swap with left: {:?}", e.1))
			.or_else(|(credit_in, _)| {
				Right::swap_exact_tokens_for_tokens(path, credit_in, amount_out_min)
			})
	}

	fn swap_tokens_for_exact_tokens(
		path: Vec<Self::AssetKind>,
		credit_in_max: Self::Credit,
		amount_out: Self::Balance,
	) -> Result<(Self::Credit, Self::Credit), (Self::Credit, sdk::sp_runtime::DispatchError)> {
		Left::swap_tokens_for_exact_tokens(path.clone(), credit_in_max, amount_out)
			.inspect_err(|e| log::debug!("Failed to swap with left: {:?}", e.1))
			.or_else(|(credit_in_max, _)| {
				Right::swap_tokens_for_exact_tokens(path, credit_in_max, amount_out)
			})
	}
}

pub struct FundAssetConversion<FundAccount, AssetRate, Fungibles> {
	_phantom: core::marker::PhantomData<(FundAccount, AssetRate, Fungibles)>,
}

impl<FundAccount, AssetRate, Fungibles> FundAssetConversion<FundAccount, AssetRate, Fungibles>
where
	FundAccount: Get<AccountId>,
	AssetRate: ConversionFromAssetBalance<Balance, Location, Balance>
		+ ConversionToAssetBalance<Balance, Location, Balance>,
	Fungibles: fungibles::Mutate<AccountId>
		+ fungibles::Balanced<AccountId>
		+ fungibles::Inspect<AccountId, AssetId = Location, Balance = Balance>,
{
	fn resolve_and_withdraw(
		credit_in: <Self as SwapCredit<AccountId>>::Credit,
		asset_out: Location,
		amount_out: Balance,
	) -> Result<<Self as SwapCredit<AccountId>>::Credit, <Self as SwapCredit<AccountId>>::Credit> {
		use sdk::frame_support::storage::{
			transactional::with_transaction_opaque_err, TransactionOutcome,
		};
		let credit_in_opt = &mut Some(credit_in);
		let outer = with_transaction_opaque_err(|| {
			let credit_in = credit_in_opt.take().expect(
				"The transaction nesting level could be incremented, so outer will not be Err",
			);
			let inner = Self::do_resolve_and_withdraw(credit_in, asset_out, amount_out);
			if inner.is_ok() {
				TransactionOutcome::Commit(inner)
			} else {
				TransactionOutcome::Rollback(inner)
			}
		});
		match outer {
			Ok(inner) => inner,
			Err(()) => {
				Err(credit_in_opt.take()
					.expect("The transaction nesting level could NOT be incremented, so the inner will not be calculated"))
			}
		}
	}

	fn do_resolve_and_withdraw(
		credit_in: <Self as SwapCredit<AccountId>>::Credit,
		asset_out: Location,
		amount_out: Balance,
	) -> Result<<Self as SwapCredit<AccountId>>::Credit, <Self as SwapCredit<AccountId>>::Credit> {
		let Ok(credit_out) = Fungibles::withdraw(
			asset_out,
			&FundAccount::get(),
			amount_out,
			Precision::Exact,
			Preservation::Preserve,
			Fortitude::Polite,
		) else {
			return Err(credit_in);
		};

		// `credit_out` is already withdrawn. If this next resolve does not succeed, we need to roll back all storage changes.
		Fungibles::resolve(&FundAccount::get(), credit_in).map(|_| credit_out)
	}
}

impl<FundAccount, AssetRate, Fungibles> QuotePrice
	for FundAssetConversion<FundAccount, AssetRate, Fungibles>
where
	AssetRate: ConversionFromAssetBalance<Balance, Location, Balance>
		+ ConversionToAssetBalance<Balance, Location, Balance>,
{
	type Balance = Balance;
	type AssetKind = Location;

	fn quote_price_tokens_for_exact_tokens(
		in_asset: Self::AssetKind,
		out_asset: Self::AssetKind,
		out_asset_amount: Self::Balance,
		_include_fee: bool,
	) -> Option<Self::Balance> {
		match (in_asset, out_asset) {
			(in_asset, out_asset) if in_asset == Location::here() && out_asset == in_asset => {
				Some(out_asset_amount)
			},
			(in_asset, out_asset) if in_asset == Location::here() => {
				AssetRate::from_asset_balance(out_asset_amount, out_asset).ok()
			},
			(in_asset, out_asset) if out_asset == Location::here() => {
				AssetRate::to_asset_balance(out_asset_amount, in_asset).ok()
			},
			_ => None,
		}
	}

	fn quote_price_exact_tokens_for_tokens(
		in_asset: Self::AssetKind,
		out_asset: Self::AssetKind,
		in_asset_amount: Self::Balance,
		_include_fee: bool,
	) -> Option<Self::Balance> {
		match (in_asset, out_asset) {
			(in_asset, out_asset) if in_asset == Location::here() && out_asset == in_asset => {
				Some(in_asset_amount)
			},
			(in_asset, out_asset) if in_asset == Location::here() => {
				AssetRate::to_asset_balance(in_asset_amount, out_asset).ok()
			},
			(in_asset, out_asset) if out_asset == Location::here() => {
				AssetRate::from_asset_balance(in_asset_amount, in_asset).ok()
			},
			_ => None,
		}
	}
}

impl<FundAccount, AssetRate, Fungibles> SwapCredit<AccountId>
	for FundAssetConversion<FundAccount, AssetRate, Fungibles>
where
	FundAccount: Get<AccountId>,
	AssetRate: ConversionFromAssetBalance<Balance, Location, Balance>
		+ ConversionToAssetBalance<Balance, Location, Balance>,
	Fungibles: fungibles::Mutate<AccountId>
		+ fungibles::Balanced<AccountId>
		+ fungibles::Inspect<AccountId, AssetId = Location, Balance = Balance>,
{
	type Balance = Balance;
	type AssetKind = Location;
	type Credit = Credit<AccountId, Fungibles>;

	fn max_path_len() -> u32 {
		2
	}

	fn swap_exact_tokens_for_tokens(
		path: Vec<Self::AssetKind>,
		credit_in: Self::Credit,
		amount_out_min: Option<Self::Balance>,
	) -> Result<Self::Credit, (Self::Credit, DispatchError)> {
		let Ok([asset_in, asset_out]) = TryInto::<[Location; 2]>::try_into(path) else {
			return Err((credit_in, DispatchError::Other("asset swap path is invalid")));
		};

		let Some(amount_out) = Self::quote_price_exact_tokens_for_tokens(
			asset_in,
			asset_out.clone(),
			credit_in.peek(),
			false,
		) else {
			return Err((credit_in, DispatchError::Other("asset swap path is invalid")));
		};

		if amount_out_min.is_some_and(|min| amount_out < min) {
			return Err((credit_in, DispatchError::Other("slippage exceeded")));
		}

		Self::resolve_and_withdraw(credit_in, asset_out, amount_out)
			.map_err(|credit_in| (credit_in, DispatchError::Other("could not transact assets")))
	}

	fn swap_tokens_for_exact_tokens(
		path: Vec<Self::AssetKind>,
		credit_in_max: Self::Credit,
		amount_out: Self::Balance,
	) -> Result<(Self::Credit, Self::Credit), (Self::Credit, DispatchError)> {
		let Ok([asset_in, asset_out]) = TryInto::<[Location; 2]>::try_into(path) else {
			return Err((credit_in_max, DispatchError::Other("asset swap path is invalid")));
		};

		let Some(amount_in) = Self::quote_price_tokens_for_exact_tokens(
			asset_in,
			asset_out.clone(),
			amount_out,
			false,
		) else {
			return Err((credit_in_max, DispatchError::Other("asset swap path is invalid")));
		};

		if amount_in > credit_in_max.peek() {
			return Err((credit_in_max, DispatchError::Other("slippage exceeded")));
		}

		let (credit_in, credit_remaining) = credit_in_max.split(amount_in);

		Self::resolve_and_withdraw(credit_in, asset_out, amount_out)
			.map(|credit_out| (credit_out, credit_remaining))
			.map_err(|credit_in| (credit_in, DispatchError::Other("could not transact assets")))
	}
}

#[cfg(test)]
mod tests {
	use frame_support::pallet_prelude::CheckedDiv;
	use sdk::sp_runtime::{FixedPointNumber, FixedU128};

	use crate::xcm_config::{DotLocation, HereLocation};

	use super::*;

	struct MockAssetRate;

	impl MockAssetRate {
		pub const RATE: FixedU128 = FixedU128::from_u32(1_300_000_000);
	}

	impl ConversionFromAssetBalance<Balance, Location, Balance> for MockAssetRate {
		type Error = pallet_asset_conversion::Error<crate::Runtime>;

		fn from_asset_balance(
			balance: Balance,
			_asset_id: Location,
		) -> Result<Balance, Self::Error> {
			Ok(Self::RATE.saturating_mul_int(balance))
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn ensure_successful(_asset_id: Location) {
			todo!()
		}
	}

	impl ConversionToAssetBalance<Balance, Location, Balance> for MockAssetRate {
		type Error = pallet_asset_conversion::Error<crate::Runtime>;

		fn to_asset_balance(balance: Balance, _asset_id: Location) -> Result<Balance, Self::Error> {
			Ok(FixedU128::from_u32(1)
				.checked_div(&Self::RATE)
				.ok_or(pallet_asset_conversion::Error::<crate::Runtime>::Overflow)?
				.saturating_mul_int(balance))
		}
	}

	type AssetConversion = FundAssetConversion<
		funds::financial_fund::Account,
		MockAssetRate,
		/* This is not used in `QuotePrice`: */ NativeAndAssets,
	>;

	// When providing an asset-rate in typescript code or
	// on polkadot js we have to provide the inner representation of `FixedU128`
	// rather then the integer/rational we desire to represent.
	//
	// The inner representation for integers is the integer scaled by 10^18.
	#[test]
	fn asset_rate_representation_assumption_holds() {
		let a = FixedU128::saturating_from_integer(42);
		let b = FixedU128::from_inner(42 * 1_000_000_000_000_000_000);

		assert_eq!(a, b);
	}

	#[test]
	fn quote_price_tokens_for_exact_tokens() {
		let tiles = 13_000_000_000_000_000_000; // 13 MOS
		let plancks = 10_000_000_000; // 1 DOT

		let actual_tiles = AssetConversion::quote_price_tokens_for_exact_tokens(
			HereLocation::get(),
			DotLocation::get(),
			plancks,
			false,
		)
		.unwrap();

		assert_eq!(tiles, actual_tiles);
	}

	#[test]
	fn quote_price_exact_tokens_for_tokens() {
		let tiles = 13_000_000_000_000_000_000; // 13 MOS
		let plancks = 10_000_000_000; // 1 DOT

		let actual_plancks = AssetConversion::quote_price_exact_tokens_for_tokens(
			HereLocation::get(),
			DotLocation::get(),
			tiles,
			false,
		)
		.unwrap();

		assert_eq!(plancks - actual_plancks, 3);
	}
}
