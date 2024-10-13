#![allow(non_camel_case_types)]

pub use SpecId::*;

#[cfg(not(feature = "optimism"))]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, enumn::N)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpecId {
    FRONTIER = 0, FRONTIER_THAWING = 1, HOMESTEAD = 2, DAO_FORK = 3, TANGERINE = 4,
    SPURIOUS_DRAGON = 5, BYZANTIUM = 6, CONSTANTINOPLE = 7, PETERSBURG = 8, ISTANBUL = 9,
    MUIR_GLACIER = 10, BERLIN = 11, LONDON = 12, ARROW_GLACIER = 13, GRAY_GLACIER = 14,
    MERGE = 15, SHANGHAI = 16, CANCUN = 17, PRAGUE = 18, #[default] LATEST = u8::MAX,
}

#[cfg(feature = "optimism")]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, enumn::N)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SpecId {
    FRONTIER = 0, FRONTIER_THAWING = 1, HOMESTEAD = 2, DAO_FORK = 3, TANGERINE = 4,
    SPURIOUS_DRAGON = 5, BYZANTIUM = 6, CONSTANTINOPLE = 7, PETERSBURG = 8, ISTANBUL = 9,
    MUIR_GLACIER = 10, BERLIN = 11, LONDON = 12, ARROW_GLACIER = 13, GRAY_GLACIER = 14,
    MERGE = 15, BEDROCK = 16, REGOLITH = 17, SHANGHAI = 18, CANYON = 19,
    CANCUN = 20, ECOTONE = 21, PRAGUE = 22, #[default] LATEST = u8::MAX,
}

impl SpecId {
    #[inline]
    pub fn try_from_u8(spec_id: u8) -> Option<Self> { Self::n(spec_id) }

    #[inline]
    pub const fn is_enabled_in(self, other: Self) -> bool { self as u8 >= other as u8 }

    #[inline]
    pub const fn enabled(our: SpecId, other: SpecId) -> bool { our as u8 >= other as u8 }
}

impl From<&str> for SpecId {
    fn from(name: &str) -> Self {
        match name {
            "Frontier" => Self::FRONTIER, "Homestead" => Self::HOMESTEAD, "Tangerine" => Self::TANGERINE,
            "Spurious" => Self::SPURIOUS_DRAGON, "Byzantium" => Self::BYZANTIUM,
            "Constantinople" => Self::CONSTANTINOPLE, "Petersburg" => Self::PETERSBURG,
            "Istanbul" => Self::ISTANBUL, "MuirGlacier" => Self::MUIR_GLACIER, "Berlin" => Self::BERLIN,
            "London" => Self::LONDON, "Merge" => Self::MERGE, "Shanghai" => Self::SHANGHAI,
            "Cancun" => Self::CANCUN, "Prague" => Self::PRAGUE,
            #[cfg(feature = "optimism")]
            "Bedrock" => Self::BEDROCK,
            #[cfg(feature = "optimism")]
            "Regolith" => Self::REGOLITH,
            #[cfg(feature = "optimism")]
            "Canyon" => Self::CANYON,
            #[cfg(feature = "optimism")]
            "Ecotone" => Self::ECOTONE,
            _ => Self::LATEST,
        }
    }
}

impl From<SpecId> for &'static str {
    fn from(spec_id: SpecId) -> Self {
        match spec_id {
            SpecId::FRONTIER => "Frontier", SpecId::FRONTIER_THAWING => "Frontier Thawing",
            SpecId::HOMESTEAD => "Homestead", SpecId::DAO_FORK => "DAO Fork",
            SpecId::TANGERINE => "Tangerine", SpecId::SPURIOUS_DRAGON => "Spurious",
            SpecId::BYZANTIUM => "Byzantium", SpecId::CONSTANTINOPLE => "Constantinople",
            SpecId::PETERSBURG => "Petersburg", SpecId::ISTANBUL => "Istanbul",
            SpecId::MUIR_GLACIER => "MuirGlacier", SpecId::BERLIN => "Berlin",
            SpecId::LONDON => "London", SpecId::ARROW_GLACIER => "Arrow Glacier",
            SpecId::GRAY_GLACIER => "Gray Glacier", SpecId::MERGE => "Merge",
            SpecId::SHANGHAI => "Shanghai", SpecId::CANCUN => "Cancun", SpecId::PRAGUE => "Prague",
            #[cfg(feature = "optimism")]
            SpecId::BEDROCK => "Bedrock",
            #[cfg(feature = "optimism")]
            SpecId::REGOLITH => "Regolith",
            #[cfg(feature = "optimism")]
            SpecId::CANYON => "Canyon",
            #[cfg(feature = "optimism")]
            SpecId::ECOTONE => "Ecotone",
            SpecId::LATEST => "Latest",
        }
    }
}

pub trait Spec: Sized + 'static {
    const SPEC_ID: SpecId;
    #[inline]
    fn enabled(spec_id: SpecId) -> bool { SpecId::enabled(Self::SPEC_ID, spec_id) }
}

macro_rules! spec {
    ($spec_id:ident, $spec_name:ident) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $spec_name;
        impl Spec for $spec_name { const SPEC_ID: SpecId = $spec_id; }
    };
}

spec!(FRONTIER, FrontierSpec);
spec!(HOMESTEAD, HomesteadSpec);
spec!(TANGERINE, TangerineSpec);
spec!(SPURIOUS_DRAGON, SpuriousDragonSpec);
spec!(BYZANTIUM, ByzantiumSpec);
spec!(PETERSBURG, PetersburgSpec);
spec!(ISTANBUL, IstanbulSpec);
spec!(BERLIN, BerlinSpec);
spec!(LONDON, LondonSpec);
spec!(MERGE, MergeSpec);
spec!(SHANGHAI, ShanghaiSpec);
spec!(CANCUN, CancunSpec);
spec!(PRAGUE, PragueSpec);
spec!(LATEST, LatestSpec);

#[cfg(feature = "optimism")]
spec!(BEDROCK, BedrockSpec);
#[cfg(feature = "optimism")]
spec!(REGOLITH, RegolithSpec);
#[cfg(feature = "optimism")]
spec!(CANYON, CanyonSpec);
#[cfg(feature = "optimism")]
spec!(ECOTONE, EcotoneSpec);

#[macro_export]
macro_rules! spec_to_generic {
    ($spec_id:expr, $e:expr) => {{
        match $spec_id {
            $crate::SpecId::FRONTIER | SpecId::FRONTIER_THAWING => { use $crate::FrontierSpec as SPEC; $e },
            $crate::SpecId::HOMESTEAD | SpecId::DAO_FORK => { use $crate::HomesteadSpec as SPEC; $e },
            $crate::SpecId::TANGERINE => { use $crate::TangerineSpec as SPEC; $e },
            $crate::SpecId::SPURIOUS_DRAGON => { use $crate::SpuriousDragonSpec as SPEC; $e },
            $crate::SpecId::BYZANTIUM => { use $crate::ByzantiumSpec as SPEC; $e },
            $crate::SpecId::PETERSBURG | $crate::SpecId::CONSTANTINOPLE => { use $crate::PetersburgSpec as SPEC; $e },
            $crate::SpecId::ISTANBUL | $crate::SpecId::MUIR_GLACIER => { use $crate::IstanbulSpec as SPEC; $e },
            $crate::SpecId::BERLIN => { use $crate::BerlinSpec as SPEC; $e },
            $crate::SpecId::LONDON | $crate::SpecId::ARROW_GLACIER | $crate::SpecId::GRAY_GLACIER => { use $crate::LondonSpec as SPEC; $e },
            $crate::SpecId::MERGE => { use $crate::MergeSpec as SPEC; $e },
            $crate::SpecId::SHANGHAI => { use $crate::ShanghaiSpec as SPEC; $e },
            $crate::SpecId::CANCUN => { use $crate::CancunSpec as SPEC; $e },
            $crate::SpecId::LATEST => { use $crate::LatestSpec as SPEC; $e },
            $crate::SpecId::PRAGUE => { use $crate::PragueSpec as SPEC; $e },
            #[cfg(feature = "optimism")]
            $crate::SpecId::BEDROCK => { use $crate::BedrockSpec as SPEC; $e },
            #[cfg(feature = "optimism")]
            $crate::SpecId::REGOLITH => { use $crate::RegolithSpec as SPEC; $e },
            #[cfg(feature = "optimism")]
            $crate::SpecId::CANYON => { use $crate::CanyonSpec as SPEC; $e },
            #[cfg(feature = "optimism")]
            $crate::SpecId::ECOTONE => { use $crate::EcotoneSpec as SPEC; $e },
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_to_generic() {
        use SpecId::*;
        spec_to_generic!(FRONTIER, assert_eq!(SPEC::SPEC_ID, FRONTIER));
        spec_to_generic!(FRONTIER_THAWING, assert_eq!(SPEC::SPEC_ID, FRONTIER));
        spec_to_generic!(HOMESTEAD, assert_eq!(SPEC::SPEC_ID, HOMESTEAD));
        spec_to_generic!(DAO_FORK, assert_eq!(SPEC::SPEC_ID, HOMESTEAD));
        spec_to_generic!(TANGERINE, assert_eq!(SPEC::SPEC_ID, TANGERINE));
        spec_to_generic!(SPURIOUS_DRAGON, assert_eq!(SPEC::SPEC_ID, SPURIOUS_DRAGON));
        spec_to_generic!(BYZANTIUM, assert_eq!(SPEC::SPEC_ID, BYZANTIUM));
        spec_to_generic!(CONSTANTINOPLE, assert_eq!(SPEC::SPEC_ID, PETERSBURG));
        spec_to_generic!(PETERSBURG, assert_eq!(SPEC::SPEC_ID, PETERSBURG));
        spec_to_generic!(ISTANBUL, assert_eq!(SPEC::SPEC_ID, ISTANBUL));
        spec_to_generic!(MUIR_GLACIER, assert_eq!(SPEC::SPEC_ID, ISTANBUL));
        spec_to_generic!(BERLIN, assert_eq!(SPEC::SPEC_ID, BERLIN));
        spec_to_generic!(LONDON, assert_eq!(SPEC::SPEC_ID, LONDON));
        spec_to_generic!(ARROW_GLACIER, assert_eq!(SPEC::SPEC_ID, LONDON));
        spec_to_generic!(GRAY_GLACIER, assert_eq!(SPEC::SPEC_ID, LONDON));
        spec_to_generic!(MERGE, assert_eq!(SPEC::SPEC_ID, MERGE));
        spec_to_generic!(SHANGHAI, assert_eq!(SPEC::SPEC_ID, SHANGHAI));
        spec_to_generic!(CANCUN, assert_eq!(SPEC::SPEC_ID, CANCUN));
        spec_to_generic!(PRAGUE, assert_eq!(SPEC::SPEC_ID, PRAGUE));
        spec_to_generic!(LATEST, assert_eq!(SPEC::SPEC_ID, LATEST));
        #[cfg(feature = "optimism")]
        {
            spec_to_generic!(BEDROCK, assert_eq!(SPEC::SPEC_ID, BEDROCK));
            spec_to_generic!(REGOLITH, assert_eq!(SPEC::SPEC_ID, REGOLITH));
            spec_to_generic!(CANYON, assert_eq!(SPEC::SPEC_ID, CANYON));
        }
    }
}

#[cfg(feature = "optimism")]
#[cfg(test)]
mod optimism_tests {
    use super::*;

    #[test]
    fn test_bedrock_post_merge_hardforks() {
        assert!(BedrockSpec::enabled(SpecId::MERGE));
        assert!(!BedrockSpec::enabled(SpecId::SHANGHAI));
        assert!(!BedrockSpec::enabled(SpecId::CANCUN));
        assert!(!BedrockSpec::enabled(SpecId::LATEST));
        assert!(BedrockSpec::enabled(SpecId::BEDROCK));
        assert!(!BedrockSpec::enabled(SpecId::REGOLITH));
    }

    #[test]
    fn test_regolith_post_merge_hardforks() {
        assert!(RegolithSpec::enabled(SpecId::MERGE));
        assert!(!RegolithSpec::enabled(SpecId::SHANGHAI));
        assert!(!RegolithSpec::enabled(SpecId::CANCUN));
        assert!(!RegolithSpec::enabled(SpecId::LATEST));
        assert!(RegolithSpec::enabled(SpecId::BEDROCK));
        assert!(RegolithSpec::enabled(SpecId::REGOLITH));
    }

    #[test]
    fn test_canyon_post_merge_hardforks() {
        assert!(CanyonSpec::enabled(SpecId::MERGE));
        assert!(CanyonSpec::enabled(SpecId::SHANGHAI));
        assert!(!CanyonSpec::enabled(SpecId::CANCUN));
        assert!(!CanyonSpec::enabled(SpecId::LATEST));
        assert!(CanyonSpec::enabled(SpecId::BEDROCK));
        assert!(CanyonSpec::enabled(SpecId::REGOLITH));
        assert!(CanyonSpec::enabled(SpecId::CANYON));
    }

    #[test]
    fn test_ecotone_post_merge_hardforks() {
        assert!(EcotoneSpec::enabled(SpecId::MERGE));
        assert!(EcotoneSpec::enabled(SpecId::SHANGHAI));
        assert!(EcotoneSpec::enabled(SpecId::CANCUN));
        assert!(!EcotoneSpec::enabled(SpecId::LATEST));
        assert!(EcotoneSpec::enabled(SpecId::BEDROCK));
        assert!(EcotoneSpec::enabled(SpecId::REGOLITH));
        assert!(EcotoneSpec::enabled(SpecId::CANYON));
        assert!(EcotoneSpec::enabled(SpecId::ECOTONE));
    }

    #[test]
    fn test_spec_id_enabled() {
        assert!(SpecId::enabled(SpecId::BEDROCK, SpecId::MERGE));
        assert!(!SpecId::enabled(SpecId::BEDROCK, SpecId::SHANGHAI));
        assert!(!SpecId::enabled(SpecId::BEDROCK, SpecId::CANCUN));
        assert!(!SpecId::enabled(SpecId::BEDROCK, SpecId::LATEST));
        assert!(SpecId::enabled(SpecId::BEDROCK, SpecId::BEDROCK));
        assert!(!SpecId::enabled(SpecId::BEDROCK, SpecId::REGOLITH));

        assert!(SpecId::enabled(SpecId::REGOLITH, SpecId::MERGE));
        assert!(!SpecId::enabled(SpecId::REGOLITH, SpecId::SHANGHAI));
        assert!(!SpecId::enabled(SpecId::REGOLITH, SpecId::CANCUN));
        assert!(!SpecId::enabled(SpecId::REGOLITH, SpecId::LATEST));
        assert!(SpecId::enabled(SpecId::REGOLITH, SpecId::BEDROCK));
        assert!(SpecId::enabled(SpecId::REGOLITH, SpecId::REGOLITH));

        assert!(SpecId::enabled(SpecId::CANYON, SpecId::MERGE));
        assert!(SpecId::enabled(SpecId::CANYON, SpecId::SHANGHAI));
        assert!(!SpecId::enabled(SpecId::CANYON, SpecId::CANCUN));
        assert!(!SpecId::enabled(SpecId::CANYON, SpecId::LATEST));
        assert!(SpecId::enabled(SpecId::CANYON, SpecId::BEDROCK));
        assert!(SpecId::enabled(SpecId::CANYON, SpecId::REGOLITH));
        assert!(SpecId::enabled(SpecId::CANYON, SpecId::CANYON));

        assert!(SpecId::enabled(SpecId::ECOTONE, SpecId::MERGE));
        assert!(SpecId::enabled(SpecId::ECOTONE, SpecId::SHANGHAI));
        assert!(SpecId::enabled(SpecId::ECOTONE, SpecId::CANCUN));
        assert!(!SpecId::enabled(SpecId::ECOTONE, SpecId::LATEST));
        assert!(SpecId::enabled(SpecId::ECOTONE, SpecId::BEDROCK));
        assert!(SpecId::enabled(SpecId::ECOTONE, SpecId::REGOLITH));
        assert!(SpecId::enabled(SpecId::ECOTONE, SpecId::CANYON));
        assert!(SpecId::enabled(SpecId::ECOTONE, SpecId::ECOTONE));
    }
}
