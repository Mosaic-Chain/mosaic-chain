use sdk::{frame_support, pallet_authorship, sp_core::Get, sp_runtime};

use super::{
	funds::treasury::Account as TreasuryAccount, params, AccountId, Balance, Balanced, Balances,
	Credit, Currency, Debt, Imbalance, InvalidTransaction, OnChargeTransaction, Precision, Runtime,
	RuntimeCall, TransactionValidityError, Zero,
};

pub struct ChargeTransaction;

impl ChargeTransaction {
	fn process_fees<I: Imbalance<Balance>>(fee: I, tip: I) {
		if let Some(author) = <pallet_authorship::Pallet<Runtime>>::author() {
			let ratio = params::dynamic::transaction_payment::FeePaymentRatio::get();

			let (fee_to_author, rest) = fee.ration(ratio.validator, ratio.treasury + ratio.burn);
			let (fee_to_treasury, _burnt) = rest.ration(ratio.treasury, ratio.burn);

			let author_share = fee_to_author.merge(tip);

			let _ = Balances::deposit_creating(&author, author_share.peek());
			let _ = Balances::deposit_creating(&TreasuryAccount::get(), fee_to_treasury.peek());
		} else {
			// NOTE: I can't really imagine a scenario when we can't find the author,
			// but this behaviour seems fine in either case.

			let total = fee.merge(tip);
			let _ = Balances::deposit_creating(&TreasuryAccount::get(), total.peek());
		}
	}
}

impl OnChargeTransaction<Runtime> for ChargeTransaction {
	type Balance = Balance;
	type LiquidityInfo = Option<Credit<AccountId, Balances>>;

	fn withdraw_fee(
		who: &AccountId,
		_call: &RuntimeCall,
		_dispatch_info: &sp_runtime::traits::DispatchInfoOf<RuntimeCall>,
		fee: Self::Balance,
		_tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, frame_support::pallet_prelude::TransactionValidityError> {
		if fee.is_zero() {
			return Ok(None);
		}

		match <Balances as Balanced<AccountId>>::withdraw(
			who,
			fee,
			Precision::Exact,
			frame_support::traits::tokens::Preservation::Preserve,
			frame_support::traits::tokens::Fortitude::Polite,
		) {
			Ok(imbalance) => Ok(Some(imbalance)),
			Err(_) => Err(InvalidTransaction::Payment.into()),
		}
	}

	fn correct_and_deposit_fee(
		who: &AccountId,
		_dispatch_info: &sp_runtime::traits::DispatchInfoOf<RuntimeCall>,
		_post_info: &sp_runtime::traits::PostDispatchInfoOf<RuntimeCall>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), frame_support::pallet_prelude::TransactionValidityError> {
		if let Some(paid) = already_withdrawn {
			// Calculate how much refund we should return
			let refund_amount = paid.peek().saturating_sub(corrected_fee);

			// refund to the the account that paid the fees if it exists. otherwise, don't refund
			// anything.
			let refund_imbalance =
				if Balances::total_balance(who) > Balance::zero() && !refund_amount.is_zero() {
					Balances::deposit(who, refund_amount, Precision::BestEffort)
						.unwrap_or_else(|_| Debt::<AccountId, Balances>::zero())
				} else {
					Debt::<AccountId, Balances>::zero()
				};

			// merge the imbalance caused by paying the fees and refunding parts of it again.
			let adjusted_paid: Credit<AccountId, Balances> =
				paid.offset(refund_imbalance)
					.same()
					.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Payment))?;

			// Call someone else to handle the imbalance (fee and tip separately)
			let (tip, fee) = adjusted_paid.split(tip);

			Self::process_fees(fee, tip);
		}

		Ok(())
	}
}
