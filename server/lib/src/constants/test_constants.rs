use std::str::FromStr;

use crate::prelude::*;
use crate::value::Value;

pub(crate) const TEST_TESTGROUP_A_UUID: &str = "d2b496bd-8493-47b7-8142-f568b5cf47ee";
pub(crate) const TEST_TESTGROUP_B_UUID: &str = "8cef42bc-2cac-43e4-96b3-8f54561885ca";

lazy_static! {
    pub(crate) static ref TESTGROUP_ENTRY_A: EntryInitNew = entry_init!(
        (Attribute::Class, EntryClass::Group.to_value()),
        (Attribute::Name, Value::new_iname("testgroup_a")),
        (Attribute::Description, Value::new_utf8s("testgroup")),
        (
            Attribute::Uuid,
            Value::Uuid(
                Uuid::from_str(TEST_TESTGROUP_A_UUID).expect("test uuid we hard coded failed...")
            )
        )
    );
}
