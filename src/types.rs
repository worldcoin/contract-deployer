use serde::{Deserialize, Serialize};
use shrinkwraprs::Shrinkwrap;

macro_rules! impl_primitive_num {
    (pub struct $outer:ident($tname:ty)) => {
        #[derive(
            Debug,
            Clone,
            Copy,
            Serialize,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Deserialize,
            Shrinkwrap,
        )]
        pub struct $outer(pub $tname);

        impl std::fmt::Display for $outer {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $outer {
            type Err = <$tname as std::str::FromStr>::Err;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let n = s.parse::<$tname>()?;

                Ok(Self(n))
            }
        }
    };
}

impl_primitive_num!(pub struct GroupId(usize));
impl_primitive_num!(pub struct TreeDepth(usize));
impl_primitive_num!(pub struct BatchSize(usize));
