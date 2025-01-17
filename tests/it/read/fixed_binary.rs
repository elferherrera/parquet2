use parquet2::{deserialize::FixedLenBinaryPageState, error::Result, page::DataPage};

use super::dictionary::FixedLenByteArrayPageDict;
use super::utils::deserialize_optional;

pub fn page_to_vec(
    page: &DataPage,
    dict: Option<&FixedLenByteArrayPageDict>,
) -> Result<Vec<Option<Vec<u8>>>> {
    assert_eq!(page.descriptor.max_rep_level, 0);

    let state = FixedLenBinaryPageState::try_new(page, dict)?;

    match state {
        FixedLenBinaryPageState::Optional(validity, values) => {
            deserialize_optional(validity, values.map(|x| Ok(x.to_vec())))
        }
        FixedLenBinaryPageState::Required(values) => {
            Ok(values.map(|x| x.to_vec()).map(Some).collect())
        }
        FixedLenBinaryPageState::RequiredDictionary(dict) => dict
            .indexes
            .map(|x| x as usize)
            .map(|x| dict.dict.value(x).map(|x| x.to_vec()))
            .map(Some)
            .map(|x| x.transpose())
            .collect(),
        FixedLenBinaryPageState::OptionalDictionary(validity, dict) => {
            let values = dict
                .indexes
                .map(|x| x as usize)
                .map(|x| dict.dict.value(x).map(|x| x.to_vec()));
            deserialize_optional(validity, values)
        }
    }
}
