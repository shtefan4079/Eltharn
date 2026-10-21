use core::fmt;
use core::marker::PhantomData;
use core::str::FromStr;

use serde::Deserializer;
use serde::de::{self, Visitor};

use crate::Error;

pub(crate) fn from_str_deserializer<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr<Err = Error>,
    D: Deserializer<'de>,
{
    struct Bounded<T>(PhantomData<T>);

    impl<T> Visitor<'_> for Bounded<T>
    where
        T: FromStr<Err = Error>,
    {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a bounded validated string")
        }

        fn visit_str<E>(self, value: &str) -> Result<T, E>
        where
            E: de::Error,
        {
            value.parse().map_err(de::Error::custom)
        }
    }

    deserializer.deserialize_str(Bounded(PhantomData))
}
