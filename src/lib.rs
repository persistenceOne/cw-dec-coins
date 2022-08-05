use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::str::FromStr;

use cosmwasm_std::{Coin, StdError, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{de, Serialize};

#[derive(Serialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Coins(pub BTreeMap<String, Uint128>);

impl<'de> de::Deserialize<'de> for Coins {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Coins;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a map with non-duplicating string keys and stringified 128-bit unsigned integer values")
            }

            #[inline]
            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                let mut seen_denoms = HashSet::<String>::new();
                let mut coins = BTreeMap::<String, Uint128>::new();

                while let Some((denom, amount_str)) = access.next_entry::<String, String>()? {
                    if seen_denoms.contains(&denom) {
                        return Err(de::Error::custom(format!(
                            "failed to parse into Coins! duplicate denom: {}",
                            denom
                        )));
                    }

                    let amount = Uint128::from_str(&amount_str).map_err(|_| {
                        de::Error::custom(format!(
                            "failed to parse into Coins! invalid amount: {}",
                            amount_str
                        ))
                    })?;

                    seen_denoms.insert(denom.clone());
                    coins.insert(denom, amount);
                }

                Ok(Coins(coins))
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

impl From<Vec<Coin>> for Coins {
    fn from(coins: Vec<Coin>) -> Self {
        let map = coins.into_iter().map(|coin| (coin.denom, coin.amount)).collect();
        Self(map)
    }
}

impl From<&[Coin]> for Coins {
    fn from(coins: &[Coin]) -> Self {
        coins.to_vec().into()
    }
}

impl FromStr for Coins {
    type Err = StdError;

    fn from_str(s: &str) -> StdResult<Self> {
        let map = s
            .split(",")
            .into_iter()
            .map(|split| helpers::parse_coin_str(split))
            .collect::<StdResult<_>>()?;
        Ok(Self(map))
    }
}

impl fmt::Display for Coins {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // NOTE: The `iter` method for BTreeMap returns an Iterator where entries are already sorted
        // by key, so we don't need to sort the coins manually
        let s = self
            .0
            .iter()
            .map(|(denom, amount)| format!("{}{}", amount, denom))
            .collect::<Vec<_>>()
            .join(",");
        write!(f, "{}", s)
    }
}

impl Coins {
    pub fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub fn to_vec(&self) -> Vec<Coin> {
        self.0
            .iter()
            .map(|(denom, amount)| Coin {
                denom: denom.clone(),
                amount: *amount,
            })
            .collect()
    }

    pub fn into_vec(self) -> Vec<Coin> {
        self.0
            .into_iter()
            .map(|(denom, amount)| Coin {
                denom,
                amount,
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub mod helpers {
    use std::any::type_name;
    use std::str::FromStr;

    use cosmwasm_std::{Coin, StdError, StdResult, Uint128};

    /// `cosmwasm_std::Coin` does not implement `FromStr`, so we have do it ourselves
    ///
    /// Parsing the string with regex doesn't work, because the resulting wasm binary would be too big
    /// from including the `regex` library.
    ///
    /// If the binary size is not a concern, here's an example:
    /// https://github.com/PFC-Validator/terra-rust/blob/v1.1.8/terra-rust-api/src/client/core_types.rs#L34-L55
    ///
    /// We opt for the following solution: enumerate characters in the string, and break before the
    /// first non-number character. Split the string at that index.
    ///
    /// This assumes the denom never starts with a number, which is the case:
    /// https://github.com/cosmos/cosmos-sdk/blob/v0.46.0/types/coin.go#L854-L856
    pub fn parse_coin_str(s: &str) -> StdResult<(String, Uint128)> {
        for (i, c) in s.chars().enumerate() {
            if c.is_alphabetic() {
                let amount = Uint128::from_str(&s[..i])?;
                let denom = String::from(&s[i..]);
                return Ok((denom, amount));
            }
        }

        Err(StdError::parse_err(type_name::<Coin>(), format!("Invalid coin string ({})", s)))
    }
}
