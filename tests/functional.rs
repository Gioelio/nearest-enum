//! Behavioral tests for `#[derive(Nearest)]`.
//!
//! Adjust the crate name below if your proc-macro crate isn't `nearest_macro`.
use nearest_enum::Nearest;

// ---------------------------------------------------------------------
// Basic: no unit, no family
// ---------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug, Nearest)]
enum Basic {
    #[nearest(0)]
    Zero,
    #[nearest(10)]
    Ten,
    #[nearest(20)]
    Twenty,
}

#[test]
fn basic_nearest_exact_match() {
    assert_eq!(Basic::nearest(0), Basic::Zero);
    assert_eq!(Basic::nearest(10), Basic::Ten);
    assert_eq!(Basic::nearest(20), Basic::Twenty);
}

#[test]
fn basic_nearest_rounds_to_closer_value() {
    assert_eq!(Basic::nearest(4), Basic::Zero); // |4-0|=4 < |4-10|=6
    assert_eq!(Basic::nearest(18), Basic::Twenty); // |18-20|=2 < |18-10|=8
}

#[test]
fn basic_nearest_ties_favor_first_declared() {
    // 5 is equidistant from Zero (0) and Ten (10); Zero is declared first.
    assert_eq!(Basic::nearest(5), Basic::Zero);
    // 15 is equidistant from Ten (10) and Twenty (20); Ten is declared first.
    assert_eq!(Basic::nearest(15), Basic::Ten);
}

#[test]
fn basic_nearest_beyond_max_saturates_to_closest_i_e_max() {
    assert_eq!(Basic::nearest(1000), Basic::Twenty);
}

#[test]
fn basic_exact_only_matches_declared_values() {
    assert_eq!(Basic::exact(10), Some(Basic::Ten));
    assert_eq!(Basic::exact(11), None);
    assert_eq!(Basic::exact(0), Some(Basic::Zero));
}

#[test]
fn basic_ceil_picks_smallest_value_at_or_above_target() {
    assert_eq!(Basic::ceil(0), Basic::Zero);
    assert_eq!(Basic::ceil(1), Basic::Ten);
    assert_eq!(Basic::ceil(10), Basic::Ten);
    assert_eq!(Basic::ceil(11), Basic::Twenty);
    assert_eq!(Basic::ceil(20), Basic::Twenty);
}

#[test]
fn basic_ceil_beyond_max_saturates_to_max() {
    assert_eq!(Basic::ceil(1_000_000), Basic::Twenty);
}

// ---------------------------------------------------------------------
// Unit suffix: fn names become nearest_mhz / exact_mhz / ceil_mhz
// ---------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug, Nearest)]
#[nearest(unit = "mhz")]
enum Odr {
    #[nearest(0)]
    Off,
    #[nearest(1_875)]
    Hz1_875,
    #[nearest(3_750)]
    Hz3_750,
}

#[test]
fn unit_suffixed_fn_names_work() {
    assert_eq!(Odr::nearest_mhz(1_900), Odr::Hz1_875);
    assert_eq!(Odr::exact_mhz(3_750), Some(Odr::Hz3_750));
    assert_eq!(Odr::exact_mhz(3_751), None);
    assert_eq!(Odr::ceil_mhz(2_000), Odr::Hz3_750);
}

#[test]
fn unit_suffixed_fns_are_const_evaluable() {
    const CHOSEN: Odr = Odr::ceil_mhz(1_000);
    assert_eq!(CHOSEN, Odr::Hz1_875);
}

// ---------------------------------------------------------------------
// ty override: u64 to avoid overflow
// ---------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug, Nearest)]
#[nearest(ty = "u64", unit = "raw")]
enum BigVals {
    #[nearest(0)]
    Zero,
    #[nearest(5_000_000_000)]
    FiveBillion,
}

#[test]
fn ty_override_handles_values_beyond_u32() {
    assert_eq!(BigVals::nearest_raw(4_000_000_000), BigVals::FiveBillion);
    assert_eq!(BigVals::nearest_raw(100), BigVals::Zero);
    assert_eq!(BigVals::exact_raw(5_000_000_000), Some(BigVals::FiveBillion));
}

// ---------------------------------------------------------------------
// `off`: excluded from nearest/ceil unless target is exactly 0
// ---------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug, Nearest)]
#[nearest(unit = "mv")]
enum OffEnum {
    #[nearest(off)]
    Off,
    #[nearest(1_000)]
    OneVolt,
    #[nearest(2_000)]
    TwoVolts,
}

#[test]
fn off_is_chosen_only_when_target_is_exactly_zero() {
    assert_eq!(OffEnum::nearest_mv(0), OffEnum::Off);
}

#[test]
fn off_is_excluded_from_nearest_for_nonzero_targets() {
    // Without the off-skip, 1 would still resolve to OneVolt anyway (closest),
    // so also check a value where Off would otherwise "win" the diff race.
    assert_eq!(OffEnum::nearest_mv(1), OffEnum::OneVolt);
    assert_eq!(OffEnum::nearest_mv(400), OffEnum::OneVolt); // not Off, despite Off being numerically close-ish
}

#[test]
fn off_nearest_tie_break_among_remaining_entries() {
    // Equidistant between OneVolt (1000) and TwoVolts (2000); OneVolt declared first.
    assert_eq!(OffEnum::nearest_mv(1_500), OffEnum::OneVolt);
}

#[test]
fn off_exact_only_matches_at_zero() {
    assert_eq!(OffEnum::exact_mv(0), Some(OffEnum::Off));
    assert_eq!(OffEnum::exact_mv(1_000), Some(OffEnum::OneVolt));
}

#[test]
fn off_ceil_excluded_unless_target_zero() {
    assert_eq!(OffEnum::ceil_mv(0), OffEnum::Off);
    assert_eq!(OffEnum::ceil_mv(1), OffEnum::OneVolt);
    assert_eq!(OffEnum::ceil_mv(2_001), OffEnum::TwoVolts); // saturates, Off never wins here
}

// ---------------------------------------------------------------------
// `off` combined with families
// ---------------------------------------------------------------------
#[derive(Clone, Copy, PartialEq, Eq, Debug, Nearest)]
#[nearest(unit = "mv")]
enum OffFamEnum {
    #[nearest(off)]
    Off,
    #[nearest(1_000, family = "a")]
    A1000,
    #[nearest(2_000, family = "b")]
    B2000,
}

#[test]
fn off_is_still_family_base_and_respected_across_families() {
    assert_eq!(OffFamEnum::nearest_mv(0, OffFamEnumFamily::A), OffFamEnum::Off);
    assert_eq!(OffFamEnum::nearest_mv(0, OffFamEnumFamily::B), OffFamEnum::Off);
    assert_eq!(OffFamEnum::nearest_mv(1, OffFamEnumFamily::A), OffFamEnum::A1000);
    // family B can't see A1000 even discounting Off.
    assert_eq!(OffFamEnum::nearest_mv(1, OffFamEnumFamily::B), OffFamEnum::B2000);
}

fn main() {}
