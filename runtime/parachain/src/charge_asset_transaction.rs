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
	configs::parachain::NativeAndAssets, funds, xcm_config, AccountId, AssetConversion, Balance,
};
use funds::treasury::Account as TreasuryAccount;

pub type ChargeAssetTransaction = SwapAssetAdapter<
	xcm_config::HereLocation,
	NativeAndAssets,
	AssetConverterChain<
		FundAssetConversion<funds::financial_fund::Account, Foo, NativeAndAssets>,
		AssetConversion,
	>,
	ResolveAssetTo<TreasuryAccount, NativeAndAssets>,
>;

pub struct Foo;

impl ConversionFromAssetBalance<Balance, Location, Balance> for Foo {
	type Error = ();

	fn from_asset_balance(_balance: Balance, _asset_id: Location) -> Result<Balance, Self::Error> {
		todo!()
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(_asset_id: Location) {
		todo!()
	}
}

impl ConversionToAssetBalance<Balance, Location, Balance> for Foo {
	type Error = ();

	fn to_asset_balance(_balance: Balance, _asset_id: Location) -> Result<Balance, Self::Error> {
		todo!()
	}
}

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
		Left::swap_exact_tokens_for_tokens(path.clone(), credit_in, amount_out_min).or_else(
			|(credit_in, _)| Right::swap_exact_tokens_for_tokens(path, credit_in, amount_out_min),
		)
	}

	fn swap_tokens_for_exact_tokens(
		path: Vec<Self::AssetKind>,
		credit_in_max: Self::Credit,
		amount_out: Self::Balance,
	) -> Result<(Self::Credit, Self::Credit), (Self::Credit, sdk::sp_runtime::DispatchError)> {
		Left::swap_tokens_for_exact_tokens(path.clone(), credit_in_max, amount_out).or_else(
			|(credit_in_max, _)| {
				Right::swap_tokens_for_exact_tokens(path, credit_in_max, amount_out)
			},
		)
	}
}

pub struct FundAssetConversion<FundAccount, AssetRate, Fungibles> {
	_phantom: core::marker::PhantomData<(FundAccount, AssetRate, Fungibles)>,
}

impl<FundAccount, AssetRate, Fungibles> FundAssetConversion<FundAccount, AssetRate, Fungibles> {
	fn resolve_and_withdraw(
		credit_in: <Self as SwapCredit<AccountId>>::Credit,
		asset_out: Location,
		amount_out: Balance,
	) -> Result<<Self as SwapCredit<AccountId>>::Credit, <Self as SwapCredit<AccountId>>::Credit>
	where
		FundAccount: Get<AccountId>,
		AssetRate: ConversionFromAssetBalance<Balance, Location, Balance>
			+ ConversionToAssetBalance<Balance, Location, Balance>,
		Fungibles: fungibles::Mutate<AccountId>
			+ fungibles::Balanced<AccountId>
			+ fungibles::Inspect<AccountId, AssetId = Location, Balance = Balance>,
	{
		use sdk::frame_support::storage::{
			transactional::with_transaction_opaque_err, TransactionOutcome,
		};
		let credit_in_opt = &mut Some(credit_in);
		let outer = with_transaction_opaque_err(|| {
			let inner = (|| {
				let credit_in = credit_in_opt.take().expect(
					"The transaction nesting level could be incremented, so outer will not be Err",
				);
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
				Fungibles::resolve(&FundAccount::get(), credit_in).map(|_| credit_out)
			})();
			if inner.is_ok() {
				TransactionOutcome::Commit(inner)
			} else {
				TransactionOutcome::Rollback(inner)
			}
		});
		match outer {
			Ok(inner) => inner,
			Err(()) => {
				let credit_in = credit_in_opt.take()
					.expect("The transaction nesting level could NOT be incremented, so the inner will not be calculated");
				Err(credit_in)
			},
		}
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
		asset1: Self::AssetKind,
		asset2: Self::AssetKind,
		amount2: Self::Balance,
		_include_fee: bool,
	) -> Option<Self::Balance> {
		match (asset1, asset2) {
			(left, right) if left == Location::here() && right == left => Some(amount2),
			(left, right) if left == Location::here() => {
				AssetRate::to_asset_balance(amount2, right).ok()
			},
			(left, right) if right == Location::here() => {
				AssetRate::from_asset_balance(amount2, left).ok()
			},
			_ => None,
		}
	}

	fn quote_price_exact_tokens_for_tokens(
		asset1: Self::AssetKind,
		asset2: Self::AssetKind,
		amount1: Self::Balance,
		include_fee: bool,
	) -> Option<Self::Balance> {
		// No slippage when using fund for asset conversion
		Self::quote_price_tokens_for_exact_tokens(asset1, asset2, amount1, include_fee)
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
