mod calc;
mod constants;

pub use calc::*;
pub use constants::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Gas {
    limit: u64,
    remaining: u64,
    refunded: i64,
}

impl Gas {
    #[inline]
    pub const fn new(limit: u64) -> Self {
        Self {
            limit,
            remaining: limit,
            refunded: 0,
        }
    }

    #[inline]
    pub const fn new_spent(limit: u64) -> Self {
        Self {
            limit,
            remaining: 0,
            refunded: 0,
        }
    }

    #[inline]
    pub const fn limit(&self) -> u64 {
        self.limit
    }

    #[inline]
    pub const fn refunded(&self) -> i64 {
        self.refunded
    }

    #[inline]
    pub const fn spent(&self) -> u64 {
        self.limit - self.remaining
    }

    #[inline]
    pub const fn remaining(&self) -> u64 {
        self.remaining
    }

    #[inline]
    pub fn erase_cost(&mut self, returned: u64) {
        self.remaining += returned;
    }

    #[inline]
    pub fn spend_all(&mut self) {
        self.remaining = 0;
    }

    #[inline]
    pub fn record_refund(&mut self, refund: i64) {
        self.refunded += refund;
    }

    #[inline]
    pub fn set_final_refund(&mut self, is_london: bool) {
        let max_refund_quotient = if is_london { 5 } else { 2 };
        self.refunded = (self.refunded() as u64).min(self.spent() / max_refund_quotient) as i64;
    }

    #[inline]
    pub fn set_refund(&mut self, refund: i64) {
        self.refunded = refund;
    }

    #[inline]
    #[must_use]
    pub fn record_cost(&mut self, cost: u64) -> bool {
        if let Some(new_remaining) = self.remaining.checked_sub(cost) {
            self.remaining = new_remaining;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_operations() {
        let mut gas = Gas::new(1000);
        assert_eq!(gas.limit(), 1000);
        assert_eq!(gas.remaining(), 1000);
        assert_eq!(gas.spent(), 0);

        assert!(gas.record_cost(500));
        assert_eq!(gas.remaining(), 500);
        assert_eq!(gas.spent(), 500);

        gas.record_refund(100);
        assert_eq!(gas.refunded(), 100);

        gas.set_final_refund(true);
        assert_eq!(gas.refunded(), 100);

        gas.spend_all();
        assert_eq!(gas.remaining(), 0);
        assert_eq!(gas.spent(), 1000);
    }
}
