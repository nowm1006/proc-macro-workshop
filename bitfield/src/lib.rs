// Crates that have the "proc-macro" crate type are only allowed to export
// procedural macros. So we cannot have one crate that defines procedural macros
// alongside other types of public APIs like traits and structs.
//
// For this project we are going to need a #[bitfield] macro but also a trait
// and some structs. We solve this by defining the trait and structs in this
// crate, defining the attribute macro in a separate bitfield-impl crate, and
// then re-exporting the macro from this crate so that users only have one crate
// that they need to import.
//
// From the perspective of a user of this crate, they get all the necessary APIs
// (macro, trait, struct) through the one bitfield crate.
pub use bitfield_impl::bitfield;
pub use bitfield_impl::BitfieldSpecifier;
use seq::seq;
// TODO other things
pub trait Specifier {
    const BITS: usize;
    type T;

    fn convert_from_u64(value: u64) -> Self::T;
    fn convert_to_u64(item: Self::T) -> u64;
}

impl Specifier for bool {
    const BITS: usize = 1;
    type T = bool;

    fn convert_from_u64(value: u64) -> Self::T {
        match value {
            0 => false,
            _ => true,
        }
    }

    fn convert_to_u64(item: Self::T) -> u64 {
        if item {
            1
        } else {
            0
        }
    }
}

macro_rules! define_bit_enums {
    ($ty:ty, $range:pat) => {
        seq!(N in $range {
                pub enum B~N {}

                impl Specifier for B~N {
                    const BITS: usize = N;
                    type T = $ty;

                    fn convert_from_u64(value: u64) -> Self::T {
                        value.try_into().expect("value is too large")
                    }

                    fn convert_to_u64(item: Self::T) -> u64 {
                        item.into()
                    }
                }
            }
        );
    };
}

define_bit_enums!(u8, 0..=8);
define_bit_enums!(u16, 9..=16);
define_bit_enums!(u32, 17..=32);
define_bit_enums!(u64, 33..=64);

pub mod checks {
    #[macro_export]
    macro_rules! require_multiple_of_eight {
        ($e:expr) => {
            const _:() = <<[(); $e % 8] as $crate::checks::Array>::Marker as $crate::checks::TotalSizeIsMultipleOfEightBits>::CHECK;
        };
    }

    pub enum ZeroMod8 {}
    pub enum OneMod8 {}
    pub enum TwoMod8 {}
    pub enum ThreeMod8 {}
    pub enum FourMod8 {}
    pub enum FiveMod8 {}
    pub enum SixMod8 {}
    pub enum SevenMod8 {}

    pub trait Array {
        type Marker;
    }

    impl Array for [(); 0] {
        type Marker = ZeroMod8;
    }

    impl Array for [(); 1] {
        type Marker = OneMod8;
    }

    impl Array for [(); 2] {
        type Marker = TwoMod8;
    }

    impl Array for [(); 3] {
        type Marker = ThreeMod8;
    }

    impl Array for [(); 4] {
        type Marker = FourMod8;
    }

    impl Array for [(); 5] {
        type Marker = FiveMod8;
    }

    impl Array for [(); 6] {
        type Marker = SixMod8;
    }

    impl Array for [(); 7] {
        type Marker = SevenMod8;
    }

    pub trait TotalSizeIsMultipleOfEightBits {
        const CHECK: ();
    }

    impl TotalSizeIsMultipleOfEightBits for ZeroMod8 {
        const CHECK: () = ();
    }
}
