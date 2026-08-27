# nearest-enum
[![Crates.io][crates-badge]][crates-url]
[![MIT licensed][mit-badge]][mit-url]

[crates-badge]: https://img.shields.io/crates/v/nearest-enum
[crates-url]: https://crates.io/crates/nearest-enum
[mit-badge]: https://img.shields.io/crates/l/nearest-enum
[mit-url]: https://opensource.org/licenses/MIT

nearest-enum macro ease the conversion of numbers into enum strict values. Specifying the integer on the enum variants, it produces three lookup functions for free - `nearest`, `exact`, and `ceil` - all resolved as `const fn`, allowing compile-time-known lookup (when available).

```rust
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, Default, Nearest)]
#[nearest(unit = "mhz")]
pub enum Odr {
    #[default]
    #[nearest(off)]
    Off = 0x0,
    #[nearest(1_875)]  // 1.875 Hz, in mHz
    _1_875hz = 0x1,
    #[nearest(7_500)]  // 7.5 Hz
    _7_5hz = 0x2,
    #[nearest(15_000)]
    _15hz = 0x3,
}

// Resolved entirely at compile time — this is a `const`, not a function call.
const STARTUP_ODR: Odr = Odr::ceil_mhz(1);

fn main() {
    assert_eq!(Odr::nearest_mhz(1_000), Odr::_1_875hz);
    assert_eq!(Odr::exact_mhz(7_500), Some(Odr::_7_5hz));
    assert_eq!(Odr::ceil_mhz(1), Odr::_1_875hz); // Skips Off!
}
```


## Basic usage

Add `Nearest` in derive and tag each variant with #[nearest(<integer>)]:

```rust
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, Default, Nearest)]
pub enum Gain {
    #[default]
    #[nearest(1)]
    X1 = 0x0,
    #[nearest(2)]
    X2 = 0x1,
    #[nearest(4)]
    X4 = 0x2,
    #[nearest(8)]
    X8 = 0x3,
}
```

This generates three `const fn`s on `Gain` enum:

| Function | Behavior |
|--- | --- |
| `nearest(target: u32) -> Gain` | Closest value, always return something
| `exact(target: u32) -> Option<Gain>` | Only an exact match, `None` otherwise
| `ceil(target: u32) -> Gain`          | Smallest value that is `>= target`; saturates to the max if `target` exceeds everything



### Unit

Add `unit = "..."` at the enum level and it flows both into the function name, parameter name, and in the docs.

```rust
#[derive(Clone, Copy, PartialEq, Debug, Default, Nearest)]
#[nearest(unit = "mhz")]
pub enum Odr { /* ... */ }

// -> Odr::nearest_mhz(target_mhz: u32) -> Odr
// -> Odr::exact_mhz(target_mhz: u32) -> Option<Odr>
// -> Odr::ceil_mhz(target_mhz: u32) -> Odr
```
Leave `unit` attribute off and you get the un-suffixed names.


### Integer Types

Values default to `u32`. If you need more range, override with `#[nearest(ty = "u64")]`at the enum level. 

```rust
#[derive(Clone, Copy, PartialEq, Debug, Default, Nearest)]
#[nearest(ty = "u64", unit = "hz")]
pub enum SampleRate { /* values up to u64::MAX */ }
```

### 'off' special variant

Some enums may have a 0 value, used to turn-off the device. Such value should not be used in the search system, otherwise very low values may match with 'off' instead of the lower, but still active, values. 

```rust
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, Default, Nearest)]
#[nearest(unit = "hz")]
pub enum Odr {
    #[default]
    #[nearest(off)]
    Off = 0x0,
    #[nearest(5)]
    _5hz = 0x1,
    #[nearest(10)]
    _10hz = 0x2,

}

// Odr::nearest_hz(1) -> Odr::_5hz (skips Off)
// Odr::nearest_hz(0) -> Odr::Off
```

### Variant Families

Organize enum variants into logical groupings by tagging them with family = "...".

```rust
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Debug, Default, Nearest)]
#[nearest(unit = "mhz", default_family = "ha00")]
pub enum Odr {
    #[default]
    #[nearest(off)]
    Off = 0x0, // Unfamilied: defaults to Base (universally reachable)

    #[nearest(1_875, family = "ha00")]
    _1_875hz = 0x1,
    #[nearest(30_000, family = "ha00")]
    _30hz = 0x4,

    #[nearest(15_625, family = "ha01")]
    Ha01At15_625hz = 0x13,
    #[nearest(31_250, family = "ha01")]
    Ha01At31_25hz = 0x14,
}
```
When families are used, a companion `<Enum>Family` enum is generated, and all lookup functions accept a family argument:

```rust
// Search within a specific family
Odr::nearest_mhz(30_000, OdrFamily::Ha00); // Odr::_30hz
Odr::nearest_mhz(30_000, OdrFamily::Ha01); // Odr::Ha01At31_25hz

// Search using built-in selectors
Odr::nearest_mhz(30_000, OdrFamily::Default); // Searches configured default ("ha00") + Base
Odr::nearest_mhz(30_000, OdrFamily::Any);     // Searches all families unconditionally
```

`Default` and `Any` variants helps to achieve special behavior.
- `Default` is available only if `default_family` is added as container-level attribute and it will match the values of the family chosen.
- `Any` ignores the family constaints and match with every values.

Note:
- `off` could be constained to a family, but if not it will be shared among all families.
- Once families are enabled, every non-off variant must specify a family.


## Purpose

To generalize sensors drivers, common enums with different variants, like Odr (Output Data Rate) should provide a common method to setup their configuration.

Available function for each enums are:
- `nearest_<unit> -> Self`: select the nearest value between the chosen one and available
- `exact_<unit> -> Option<Self>`: return Some only if a match between input and variants exists
- `ceil_<unit> -> Self`: used to set a minimum frequency required by the application. The sensor will set the minimum to fulfill that value.

 ### Compile-time-first

 All the generated functions are `const fn`. They are evaluated at compile time, when available. Choosing an odr by using a constant value, this doesn't generate overhead at runtime. They also could work at runtime, if the value is unknown to the compiler, but this should be a limited use case.

 ## Requirements
 
- The enum must be fieldless (unit variants only).
- The enum should derive `Clone, Copy` — the generated functions return `Self` by value out of a `&'static` table, which requires `Copy`.
- At least one variant must carry a `#[nearest(...)]` value, or the derive fails to compile with a clear message.
- A float literal in `#[nearest(...)]` is rejected at compile time with a message telling you to scale it into an integer (e.g. multiply Hz by 1000 for mHz) — this is enforced, not just a convention.

## Limitations and potential improvements

- Negative attribute literals: Negative numbers can be passed as runtime arguments (when ty is set to a signed type), but are not currently supported as values inside `#[nearest(<value>)]` attributes.
- Float support: Floating-point numbers are omitted to keep macro output lightweight and suitable for bare-metal / embedded targets. Float support may be added in a future release.
- Generated doc comments: Documentation for generated functions is static and hardcoded by the macro, with no option yet for custom per-enum doc overrides.

 ## License

[MIT](https://opensource.org/licenses/MIT)
