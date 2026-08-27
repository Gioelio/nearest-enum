use nearest_enum::Nearest;

// Family without `Default` variant used. Tested with `Any` variant

#[derive(Clone, PartialEq, Debug, Copy, Nearest)]
pub enum Odr {
    #[nearest(1, family = "low")]
    Low1,
    #[nearest(5, family = "low")]
    Low5,
    #[nearest(1, family = "medium")]
    Medium1,
    #[nearest(5, family = "medium")]
    Medium5,
    #[nearest(10, family = "medium")]
    Medium10,
    #[nearest(15, family = "medium")]
    Medium15,
    #[nearest(10, family = "high")]
    High10,
    #[nearest(15, family = "high")]
    High15,
}

#[test]
fn ceil_normal() {
    assert_eq!(Odr::ceil(1, OdrFamily::Low), Odr::Low1);
    assert_eq!(Odr::ceil(4, OdrFamily::Low), Odr::Low5);
    assert_eq!(Odr::ceil(7, OdrFamily::Low), Odr::Low5);
}

#[test]
fn nearest_normal() {
    assert_eq!(Odr::nearest(1, OdrFamily::Medium), Odr::Medium1);
    assert_eq!(Odr::nearest(2, OdrFamily::High), Odr::High10);
    assert_eq!(Odr::nearest(5, OdrFamily::Low), Odr::Low5);
    assert_eq!(Odr::nearest(9, OdrFamily::Medium), Odr::Medium10);
    assert_eq!(Odr::nearest(10, OdrFamily::Medium), Odr::Medium10);
    assert_eq!(Odr::nearest(1000, OdrFamily::Medium), Odr::Medium15);
}

#[test]
fn exact_normal() {
    assert_eq!(Odr::exact(1, OdrFamily::Low), Some(Odr::Low1));
    assert_eq!(Odr::exact(2, OdrFamily::Medium), None);
    assert_eq!(Odr::exact(5, OdrFamily::Medium), Some(Odr::Medium5));
    assert_eq!(Odr::exact(10, OdrFamily::Low), None);
}

#[test]
fn ceil_any() {
    assert_eq!(Odr::ceil(1, OdrFamily::Any), Odr::Low1);
    assert_eq!(Odr::ceil(4, OdrFamily::Any), Odr::Low5);
    assert_eq!(Odr::ceil(7, OdrFamily::Any), Odr::Medium10);
}

#[test]
fn nearest_any() {
    assert_eq!(Odr::nearest(1, OdrFamily::Any), Odr::Low1);
    assert_eq!(Odr::nearest(2, OdrFamily::Any), Odr::Low1);
    assert_eq!(Odr::nearest(5, OdrFamily::Any), Odr::Low5);
    assert_eq!(Odr::nearest(9, OdrFamily::Any), Odr::Medium10);
    assert_eq!(Odr::nearest(10, OdrFamily::Any), Odr::Medium10);
    assert_eq!(Odr::nearest(1000, OdrFamily::Any), Odr::Medium15);
}

#[test]
fn exact_any() {
    assert_eq!(Odr::exact(1, OdrFamily::Any), Some(Odr::Low1));
    assert_eq!(Odr::exact(2, OdrFamily::Any), None);
    assert_eq!(Odr::exact(5, OdrFamily::Any), Some(Odr::Low5));
    assert_eq!(Odr::exact(9, OdrFamily::Any), None);
    assert_eq!(Odr::exact(10, OdrFamily::Any), Some(Odr::Medium10));
    assert_eq!(Odr::exact(1000, OdrFamily::Any), None);
}

#[derive(Clone, PartialEq, Debug, Copy, Nearest)]
#[nearest(default_family = "medium")]
pub enum OdrDefault {
    #[nearest(1, family = "low")]
    Low1,
    #[nearest(5, family = "low")]
    Low5,
    #[nearest(1, family = "medium")]
    Medium1,
    #[nearest(5, family = "medium")]
    Medium5,
    #[nearest(10, family = "medium")]
    Medium10,
    #[nearest(15, family = "medium")]
    Medium15,
    #[nearest(10, family = "high")]
    High10,
    #[nearest(15, family = "high")]
    High15,
}


#[test]
fn ceil_default() {
    assert_eq!(OdrDefault::ceil(1, OdrDefaultFamily::Default), OdrDefault::Medium1);
    assert_eq!(OdrDefault::ceil(4, OdrDefaultFamily::Default), OdrDefault::Medium5);
    assert_eq!(OdrDefault::ceil(7, OdrDefaultFamily::Default), OdrDefault::Medium10);
}

#[test]
fn nearest_default() {
    assert_eq!(OdrDefault::nearest(1, OdrDefaultFamily::Default), OdrDefault::Medium1);
    assert_eq!(OdrDefault::nearest(2, OdrDefaultFamily::Default), OdrDefault::Medium1);
    assert_eq!(OdrDefault::nearest(5, OdrDefaultFamily::Default), OdrDefault::Medium5);
    assert_eq!(OdrDefault::nearest(9, OdrDefaultFamily::Default), OdrDefault::Medium10);
    assert_eq!(OdrDefault::nearest(10, OdrDefaultFamily::Default), OdrDefault::Medium10);
    assert_eq!(OdrDefault::nearest(1000, OdrDefaultFamily::Default), OdrDefault::Medium15);
}

#[test]
fn exact_default() {
    assert_eq!(OdrDefault::exact(1, OdrDefaultFamily::Default), Some(OdrDefault::Medium1));
    assert_eq!(OdrDefault::exact(2, OdrDefaultFamily::Default), None);
    assert_eq!(OdrDefault::exact(5, OdrDefaultFamily::Default), Some(OdrDefault::Medium5));
    assert_eq!(OdrDefault::exact(9, OdrDefaultFamily::Default), None);
    assert_eq!(OdrDefault::exact(10, OdrDefaultFamily::Default), Some(OdrDefault::Medium10));
    assert_eq!(OdrDefault::exact(1000, OdrDefaultFamily::Default), None);
}

#[derive(Clone, Copy, PartialEq, Debug, Nearest)]
pub enum SmallBig {
    #[nearest(100, family = "a")]
    Big,
    #[nearest(1, family = "b")]
    Small1,
    #[nearest(2, family = "b")]
    Small2,
}

#[test]
fn ceil_saturation_stays_within_requested_family() {
    assert_eq!(SmallBig::ceil(50, SmallBigFamily::B), SmallBig::Small2);
}

fn main() {}
