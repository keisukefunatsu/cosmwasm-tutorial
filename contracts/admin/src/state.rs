use cosmwasm_std::{Addr, Timestamp};
use cw_storage_plus::Item;
use cw_storage_plus::Map;

pub const ADMINS: Map<&Addr, Timestamp> = Map::new("admins");
pub const DONATION_DENOM: Item<String> = Item::new("donation_denom");
pub const STR_TO_INT_MAP: Map<String, u64> = Map::new("str_to_int_map");
