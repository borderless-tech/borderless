#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
/// Module to test the interaction chain contract->process->process->contract
///
/// We do this by initializing a contract with a number, and feeding that number + 1 into a process.
/// Each processing entity will add +1 to the number, and then we check if the number has increased exactly 3 times
pub mod cc_contract {
    use borderless::*;
    use events::ActionOutput;
    use serde::{Deserialize, Serialize};
    pub struct CC {
        pub number: u32,
    }
    #[doc(hidden)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate borderless as _borderless;
        #[doc(hidden)]
        #[automatically_derived]
        const fn __check_storeable<T: _borderless::__private::storage_traits::Storeable>() {}
        __check_storeable::<u32>();
        #[doc(hidden)]
        #[automatically_derived]
        const SYMBOLS: &[(&str, u64)] = &[("number", 17387067394346283556u64)];
        #[automatically_derived]
        impl _borderless::__private::storage_traits::State for CC {
            fn load() -> _borderless::Result<Self> {
                let number = <u32 as ::borderless::__private::storage_traits::Storeable>::decode(
                    17387067394346283556u64,
                );
                Ok(Self { number })
            }
            fn init(
                mut value: _borderless::serialize::Value,
            ) -> _borderless::Result<Self> {
                use _borderless::Context;
                let base_value = value
                    .get_mut("number")
                    .take()
                    .context("failed to read parse field 'number'")?;
                let number = <u32 as ::borderless::__private::storage_traits::Storeable>::parse_value(
                        base_value.clone(),
                        17387067394346283556u64,
                    )
                    .context("failed to read parse field 'number'")?;
                Ok(Self { number })
            }
            fn http_get(path: String) -> _borderless::Result<Option<String>> {
                use _borderless::Context;
                let path = path.strip_prefix('/').unwrap_or(&path);
                let (path, _query) = match path.split_once('?') {
                    Some((path, query)) => (path, Some(query)),
                    None => (path, None),
                };
                if path.is_empty() {
                    let state = <Self as _borderless::__private::storage_traits::State>::load()?;
                    let mut buf = String::with_capacity(100);
                    buf.push('{');
                    let value = <u32 as ::borderless::__private::storage_traits::ToPayload>::to_payload(
                            &state.number,
                            "",
                        )?
                        .context("failed to read parse field 'number'")?;
                    buf.push('"');
                    buf.push_str("number");
                    buf.push('"');
                    buf.push(':');
                    buf.push_str(&value);
                    buf.push(',');
                    buf.pop();
                    buf.push('}');
                    return Ok(Some(buf));
                }
                let (prefix, suffix) = match path.find('/') {
                    Some(idx) => path.split_at(idx),
                    None => (path, ""),
                };
                match prefix {
                    "number" => {
                        let value = <u32 as ::borderless::__private::storage_traits::Storeable>::decode(
                            17387067394346283556u64,
                        );
                        <u32 as ::borderless::__private::storage_traits::ToPayload>::to_payload(
                            &value,
                            suffix,
                        )
                    }
                    _ => Ok(None),
                }
            }
            fn commit(self) {
                <u32 as ::borderless::__private::storage_traits::Storeable>::commit(
                    self.number,
                    17387067394346283556u64,
                );
            }
            fn symbols() -> &'static [(&'static str, u64)] {
                SYMBOLS
            }
        }
    };
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths,
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for CC {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "CC",
                    false as usize + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "number",
                    &self.number,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths,
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for CC {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "number" => _serde::__private::Ok(__Field::__field0),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"number" => _serde::__private::Ok(__Field::__field0),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                #[automatically_derived]
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<CC>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = CC;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(__formatter, "struct CC")
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<
                            u32,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct CC with 1 element",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(CC { number: __field0 })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<u32> = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field("number"),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<u32>(&mut __map)?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("number")?
                            }
                        };
                        _serde::__private::Ok(CC { number: __field0 })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &["number"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "CC",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<CC>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for CC {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for CC {
        #[inline]
        fn eq(&self, other: &CC) -> bool {
            self.number == other.number
        }
    }
    #[automatically_derived]
    impl ::core::cmp::Eq for CC {
        #[inline]
        #[doc(hidden)]
        #[coverage(off)]
        fn assert_receiver_is_total_eq(&self) -> () {
            let _: ::core::cmp::AssertParamIsEq<u32>;
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for CC {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "CC",
                "number",
                &&self.number,
            )
        }
    }
    pub enum Sinks {}
    #[doc(hidden)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate borderless as _borderless;
        #[doc(hidden)]
        #[automatically_derived]
        const fn __check_into_action<IntoAction>()
        where
            IntoAction: TryInto<_borderless::events::CallAction>,
            <IntoAction as TryInto<
                _borderless::events::CallAction,
            >>::Error: std::fmt::Display,
        {}
        #[automatically_derived]
        impl _borderless::NamedSink for Sinks {
            fn into_action(self) -> (&'static str, _borderless::events::CallAction) {
                match self {}
            }
        }
    };
    impl CC {
        /// Sets the number - is private so you cannot call it via API
        pub fn set_number(&mut self, number: u32) {
            self.number = number;
        }
        /// Starts calling the process
        pub fn call_next(&mut self) -> Result<ActionOutput> {
            let mut out = ActionOutput::default();
            Ok(out)
        }
    }
    #[doc(hidden)]
    #[automatically_derived]
    pub(super) mod __derived {
        use super::*;
        use ::borderless::prelude::*;
        use ::borderless::__private::{
            read_field, read_register, read_string_from_register, registers::*,
            storage_keys::make_user_key, write_field, write_register,
            write_string_to_register,
        };
        #[doc(hidden)]
        #[automatically_derived]
        const ACTION_SYMBOLS: &[(&str, u32)] = &[
            ("set_number", 3908652630u32),
            ("call_next", 3001791088u32),
        ];
        #[doc(hidden)]
        #[automatically_derived]
        pub(crate) struct __SetNumberArgs {
            pub(crate) number: u32,
        }
        #[doc(hidden)]
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_qualifications,
            clippy::absolute_paths,
        )]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl _serde::Serialize for __SetNumberArgs {
                fn serialize<__S>(
                    &self,
                    __serializer: __S,
                ) -> _serde::__private::Result<__S::Ok, __S::Error>
                where
                    __S: _serde::Serializer,
                {
                    let mut __serde_state = _serde::Serializer::serialize_struct(
                        __serializer,
                        "__SetNumberArgs",
                        false as usize + 1,
                    )?;
                    _serde::ser::SerializeStruct::serialize_field(
                        &mut __serde_state,
                        "number",
                        &self.number,
                    )?;
                    _serde::ser::SerializeStruct::end(__serde_state)
                }
            }
        };
        #[doc(hidden)]
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_qualifications,
            clippy::absolute_paths,
        )]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __SetNumberArgs {
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    #[allow(non_camel_case_types)]
                    #[doc(hidden)]
                    enum __Field {
                        __field0,
                        __ignore,
                    }
                    #[doc(hidden)]
                    struct __FieldVisitor;
                    #[automatically_derived]
                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "field identifier",
                            )
                        }
                        fn visit_u64<__E>(
                            self,
                            __value: u64,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                0u64 => _serde::__private::Ok(__Field::__field0),
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                        fn visit_str<__E>(
                            self,
                            __value: &str,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                "number" => _serde::__private::Ok(__Field::__field0),
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                        fn visit_bytes<__E>(
                            self,
                            __value: &[u8],
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                b"number" => _serde::__private::Ok(__Field::__field0),
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                    }
                    #[automatically_derived]
                    impl<'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(
                            __deserializer: __D,
                        ) -> _serde::__private::Result<Self, __D::Error>
                        where
                            __D: _serde::Deserializer<'de>,
                        {
                            _serde::Deserializer::deserialize_identifier(
                                __deserializer,
                                __FieldVisitor,
                            )
                        }
                    }
                    #[doc(hidden)]
                    struct __Visitor<'de> {
                        marker: _serde::__private::PhantomData<__SetNumberArgs>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    #[automatically_derived]
                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = __SetNumberArgs;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "struct __SetNumberArgs",
                            )
                        }
                        #[inline]
                        fn visit_seq<__A>(
                            self,
                            mut __seq: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::SeqAccess<'de>,
                        {
                            let __field0 = match _serde::de::SeqAccess::next_element::<
                                u32,
                            >(&mut __seq)? {
                                _serde::__private::Some(__value) => __value,
                                _serde::__private::None => {
                                    return _serde::__private::Err(
                                        _serde::de::Error::invalid_length(
                                            0usize,
                                            &"struct __SetNumberArgs with 1 element",
                                        ),
                                    );
                                }
                            };
                            _serde::__private::Ok(__SetNumberArgs {
                                number: __field0,
                            })
                        }
                        #[inline]
                        fn visit_map<__A>(
                            self,
                            mut __map: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::MapAccess<'de>,
                        {
                            let mut __field0: _serde::__private::Option<u32> = _serde::__private::None;
                            while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                                __Field,
                            >(&mut __map)? {
                                match __key {
                                    __Field::__field0 => {
                                        if _serde::__private::Option::is_some(&__field0) {
                                            return _serde::__private::Err(
                                                <__A::Error as _serde::de::Error>::duplicate_field("number"),
                                            );
                                        }
                                        __field0 = _serde::__private::Some(
                                            _serde::de::MapAccess::next_value::<u32>(&mut __map)?,
                                        );
                                    }
                                    _ => {
                                        let _ = _serde::de::MapAccess::next_value::<
                                            _serde::de::IgnoredAny,
                                        >(&mut __map)?;
                                    }
                                }
                            }
                            let __field0 = match __field0 {
                                _serde::__private::Some(__field0) => __field0,
                                _serde::__private::None => {
                                    _serde::__private::de::missing_field("number")?
                                }
                            };
                            _serde::__private::Ok(__SetNumberArgs {
                                number: __field0,
                            })
                        }
                    }
                    #[doc(hidden)]
                    const FIELDS: &'static [&'static str] = &["number"];
                    _serde::Deserializer::deserialize_struct(
                        __deserializer,
                        "__SetNumberArgs",
                        FIELDS,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<__SetNumberArgs>,
                            lifetime: _serde::__private::PhantomData,
                        },
                    )
                }
            }
        };
        #[doc(hidden)]
        #[automatically_derived]
        pub(crate) struct __CallNextArgs {}
        #[doc(hidden)]
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_qualifications,
            clippy::absolute_paths,
        )]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl _serde::Serialize for __CallNextArgs {
                fn serialize<__S>(
                    &self,
                    __serializer: __S,
                ) -> _serde::__private::Result<__S::Ok, __S::Error>
                where
                    __S: _serde::Serializer,
                {
                    let __serde_state = _serde::Serializer::serialize_struct(
                        __serializer,
                        "__CallNextArgs",
                        false as usize,
                    )?;
                    _serde::ser::SerializeStruct::end(__serde_state)
                }
            }
        };
        #[doc(hidden)]
        #[allow(
            non_upper_case_globals,
            unused_attributes,
            unused_qualifications,
            clippy::absolute_paths,
        )]
        const _: () = {
            #[allow(unused_extern_crates, clippy::useless_attribute)]
            extern crate serde as _serde;
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __CallNextArgs {
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    #[allow(non_camel_case_types)]
                    #[doc(hidden)]
                    enum __Field {
                        __ignore,
                    }
                    #[doc(hidden)]
                    struct __FieldVisitor;
                    #[automatically_derived]
                    impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                        type Value = __Field;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "field identifier",
                            )
                        }
                        fn visit_u64<__E>(
                            self,
                            __value: u64,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                        fn visit_str<__E>(
                            self,
                            __value: &str,
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                        fn visit_bytes<__E>(
                            self,
                            __value: &[u8],
                        ) -> _serde::__private::Result<Self::Value, __E>
                        where
                            __E: _serde::de::Error,
                        {
                            match __value {
                                _ => _serde::__private::Ok(__Field::__ignore),
                            }
                        }
                    }
                    #[automatically_derived]
                    impl<'de> _serde::Deserialize<'de> for __Field {
                        #[inline]
                        fn deserialize<__D>(
                            __deserializer: __D,
                        ) -> _serde::__private::Result<Self, __D::Error>
                        where
                            __D: _serde::Deserializer<'de>,
                        {
                            _serde::Deserializer::deserialize_identifier(
                                __deserializer,
                                __FieldVisitor,
                            )
                        }
                    }
                    #[doc(hidden)]
                    struct __Visitor<'de> {
                        marker: _serde::__private::PhantomData<__CallNextArgs>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    #[automatically_derived]
                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = __CallNextArgs;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "struct __CallNextArgs",
                            )
                        }
                        #[inline]
                        fn visit_seq<__A>(
                            self,
                            _: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::SeqAccess<'de>,
                        {
                            _serde::__private::Ok(__CallNextArgs {})
                        }
                        #[inline]
                        fn visit_map<__A>(
                            self,
                            mut __map: __A,
                        ) -> _serde::__private::Result<Self::Value, __A::Error>
                        where
                            __A: _serde::de::MapAccess<'de>,
                        {
                            while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                                __Field,
                            >(&mut __map)? {
                                match __key {
                                    _ => {
                                        let _ = _serde::de::MapAccess::next_value::<
                                            _serde::de::IgnoredAny,
                                        >(&mut __map)?;
                                    }
                                }
                            }
                            _serde::__private::Ok(__CallNextArgs {})
                        }
                    }
                    #[doc(hidden)]
                    const FIELDS: &'static [&'static str] = &[];
                    _serde::Deserializer::deserialize_struct(
                        __deserializer,
                        "__CallNextArgs",
                        FIELDS,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<__CallNextArgs>,
                            lifetime: _serde::__private::PhantomData,
                        },
                    )
                }
            }
        };
        #[automatically_derived]
        fn post_action_response(path: String, payload: Vec<u8>) -> Result<CallAction> {
            let path = path.replace("-", "_");
            let path = path.strip_prefix('/').unwrap_or(&path);
            let content = String::from_utf8(payload.clone()).unwrap_or_default();
            {
                let buf = ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(format_args!("{0}", content));
                    res
                });
                ::borderless::log::print(::borderless::log::Level::Info, buf);
            };
            #[allow(unreachable_code)]
            match path {
                "" => {
                    let action = CallAction::from_bytes(&payload)
                        .context("failed to parse action")?;
                    let _match_result = match &action.method {
                        MethodOrId::ByName { method } => {
                            match method.as_str() {
                                "set_number" => {
                                    return Err(
                                        ::anyhow::__private::must_use({
                                            let error = ::anyhow::__private::format_err(
                                                format_args!(
                                                    "action \'set_number\' cannot be called via web-api",
                                                ),
                                            );
                                            error
                                        }),
                                    );
                                    let _args: __derived::__SetNumberArgs = ::borderless::serialize::from_value(
                                        action.params.clone(),
                                    )?;
                                }
                                "call_next" => {
                                    let _args: __derived::__CallNextArgs = ::borderless::serialize::from_value(
                                        action.params.clone(),
                                    )?;
                                }
                                other => {
                                    return Err(
                                        ::anyhow::__private::must_use({
                                            let error = ::anyhow::__private::format_err(
                                                format_args!("Unknown method: {0}", other),
                                            );
                                            error
                                        }),
                                    );
                                }
                            }
                        }
                        MethodOrId::ById { method_id } => {
                            match method_id {
                                3908652630u32 => {
                                    return Err(
                                        ::anyhow::__private::must_use({
                                            let error = ::anyhow::__private::format_err(
                                                format_args!(
                                                    "action \'set_number\' cannot be called via web-api",
                                                ),
                                            );
                                            error
                                        }),
                                    );
                                    let _args: __derived::__SetNumberArgs = ::borderless::serialize::from_value(
                                        action.params.clone(),
                                    )?;
                                }
                                3001791088u32 => {
                                    let _args: __derived::__CallNextArgs = ::borderless::serialize::from_value(
                                        action.params.clone(),
                                    )?;
                                }
                                other => {
                                    return Err(
                                        ::anyhow::__private::must_use({
                                            let error = ::anyhow::__private::format_err(
                                                format_args!("Unknown method-id: 0x{0:04x}", other),
                                            );
                                            error
                                        }),
                                    );
                                }
                            }
                        }
                    };
                    Ok(action)
                }
                "set_number" => {
                    return Err(
                        ::anyhow::__private::must_use({
                            let error = ::anyhow::__private::format_err(
                                format_args!(
                                    "action \'set_number\' cannot be called via web-api",
                                ),
                            );
                            error
                        }),
                    );
                    let _args: __derived::__SetNumberArgs = ::borderless::serialize::from_slice(
                        &payload,
                    )?;
                    let value = ::borderless::serialize::to_value(&_args)?;
                    Ok(CallAction::by_method("set_number", value))
                }
                "call_next" => {
                    let _args: __derived::__CallNextArgs = ::borderless::serialize::from_slice(
                        &payload,
                    )?;
                    let value = ::borderless::serialize::to_value(&_args)?;
                    Ok(CallAction::by_method("call_next", value))
                }
                other => {
                    Err(
                        ::anyhow::__private::must_use({
                            let error = ::anyhow::__private::format_err(
                                format_args!("unknown method: {0}", other),
                            );
                            error
                        }),
                    )
                }
            }
        }
        #[automatically_derived]
        pub(crate) fn get_symbols() -> Result<()> {
            let symbols = Symbols::from_symbols(
                <CC as ::borderless::__private::storage_traits::State>::symbols(),
                ACTION_SYMBOLS,
            );
            let bytes = symbols.to_bytes()?;
            write_register(REGISTER_OUTPUT, &bytes);
            Ok(())
        }
        #[automatically_derived]
        pub(crate) fn exec_txn() -> Result<()> {
            let input = read_register(REGISTER_INPUT).context("missing input register")?;
            let action = CallAction::from_bytes(&input)?;
            let s = action.pretty_print()?;
            {
                let buf = ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(format_args!("{0}", s));
                    res
                });
                ::borderless::log::print(::borderless::log::Level::Info, buf);
            };
            let mut state = <CC as ::borderless::__private::storage_traits::State>::load()?;
            let _match_result = match &action.method {
                MethodOrId::ByName { method } => {
                    match method.as_str() {
                        "set_number" => {
                            let args: __derived::__SetNumberArgs = ::borderless::serialize::from_value(
                                action.params,
                            )?;
                            let result = CC::set_number(&mut state, args.number);
                            <() as ::borderless::events::ActionOutEvent>::convert_out_events(
                                result,
                            )
                        }
                        "call_next" => {
                            let result = CC::call_next(&mut state);
                            <Result<
                                ActionOutput,
                            > as ::borderless::events::ActionOutEvent>::convert_out_events(
                                result,
                            )
                        }
                        other => {
                            return Err(
                                ::anyhow::__private::must_use({
                                    let error = ::anyhow::__private::format_err(
                                        format_args!("Unknown method: {0}", other),
                                    );
                                    error
                                }),
                            );
                        }
                    }
                }
                MethodOrId::ById { method_id } => {
                    match method_id {
                        3908652630u32 => {
                            let args: __derived::__SetNumberArgs = ::borderless::serialize::from_value(
                                action.params,
                            )?;
                            let result = CC::set_number(&mut state, args.number);
                            <() as ::borderless::events::ActionOutEvent>::convert_out_events(
                                result,
                            )
                        }
                        3001791088u32 => {
                            let result = CC::call_next(&mut state);
                            <Result<
                                ActionOutput,
                            > as ::borderless::events::ActionOutEvent>::convert_out_events(
                                result,
                            )
                        }
                        other => {
                            return Err(
                                ::anyhow::__private::must_use({
                                    let error = ::anyhow::__private::format_err(
                                        format_args!("Unknown method-id: 0x{0:04x}", other),
                                    );
                                    error
                                }),
                            );
                        }
                    }
                }
            };
            let events = _match_result?;
            if !events.is_empty() {
                let bytes = events.to_bytes()?;
                write_register(REGISTER_OUTPUT, &bytes);
            }
            <CC as ::borderless::__private::storage_traits::State>::commit(state);
            Ok(())
        }
        #[automatically_derived]
        pub(crate) fn exec_introduction() -> Result<()> {
            let input = read_register(REGISTER_INPUT).context("missing input register")?;
            let introduction = Introduction::from_bytes(&input)?;
            let s = introduction.pretty_print()?;
            {
                let buf = ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(format_args!("{0}", s));
                    res
                });
                ::borderless::log::print(::borderless::log::Level::Info, buf);
            };
            let state = <CC as ::borderless::__private::storage_traits::State>::init(
                introduction.initial_state,
            )?;
            <CC as ::borderless::__private::storage_traits::State>::commit(state);
            Ok(())
        }
        #[automatically_derived]
        pub(crate) fn exec_revocation() -> Result<()> {
            let input = read_register(REGISTER_INPUT).context("missing input register")?;
            let r = Revocation::from_bytes(&input)?;
            {
                let buf = ::alloc::__export::must_use({
                    let res = ::alloc::fmt::format(
                        format_args!("Revoked contract. Reason: {0}", r.reason),
                    );
                    res
                });
                ::borderless::log::print(::borderless::log::Level::Info, buf);
            };
            Ok(())
        }
        #[automatically_derived]
        pub(crate) fn exec_get_state() -> Result<()> {
            let path = read_string_from_register(REGISTER_INPUT_HTTP_PATH)
                .context("missing http-path")?;
            let result = <CC as ::borderless::__private::storage_traits::State>::http_get(
                path,
            )?;
            let status: u16 = if result.is_some() { 200 } else { 404 };
            let payload = result.unwrap_or_default();
            write_register(REGISTER_OUTPUT_HTTP_STATUS, status.to_be_bytes());
            write_string_to_register(REGISTER_OUTPUT_HTTP_RESULT, payload);
            Ok(())
        }
        #[automatically_derived]
        pub(crate) fn exec_post_action() -> Result<()> {
            let path = read_string_from_register(REGISTER_INPUT_HTTP_PATH)
                .context("missing http-path")?;
            let payload = read_register(REGISTER_INPUT_HTTP_PAYLOAD)
                .context("missing http-payload")?;
            match post_action_response(path, payload) {
                Ok(action) => {
                    write_register(REGISTER_OUTPUT_HTTP_STATUS, 200u16.to_be_bytes());
                    write_register(REGISTER_OUTPUT_HTTP_RESULT, action.to_bytes()?);
                }
                Err(e) => {
                    write_register(REGISTER_OUTPUT_HTTP_STATUS, 400u16.to_be_bytes());
                    write_string_to_register(REGISTER_OUTPUT_HTTP_RESULT, e.to_string());
                }
            };
            Ok(())
        }
    }
    pub(super) mod actions {
        use super::__derived::*;
        #[allow(private_interfaces)]
        pub enum Actions {
            SetNumber { number: u32 },
            CallNext {},
        }
        #[automatically_derived]
        impl TryFrom<Actions> for ::borderless::events::CallAction {
            type Error = ::borderless::serialize::Error;
            fn try_from(
                value: Actions,
            ) -> ::std::result::Result<::borderless::events::CallAction, Self::Error> {
                let action = match value {
                    Actions::SetNumber { number } => {
                        let args = __SetNumberArgs { number };
                        let args_value = ::borderless::serialize::to_value(&args)?;
                        ::borderless::events::CallAction::by_method(
                            "set_number",
                            args_value,
                        )
                    }
                    Actions::CallNext {} => {
                        let args = __CallNextArgs {};
                        let args_value = ::borderless::serialize::to_value(&args)?;
                        ::borderless::events::CallAction::by_method(
                            "call_next",
                            args_value,
                        )
                    }
                };
                Ok(action)
            }
        }
    }
}
#[no_mangle]
#[automatically_derived]
pub extern "C" fn process_transaction() {
    let result = cc_contract::__derived::exec_txn();
    match result {
        Ok(()) => {
            let buf = ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!("execution successful"));
                res
            });
            ::borderless::log::print(::borderless::log::Level::Info, buf);
        }
        Err(e) => {
            let buf = ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(
                    format_args!("execution failed: {0:?}", e),
                );
                res
            });
            ::borderless::log::print(::borderless::log::Level::Error, buf);
        }
    }
}
#[no_mangle]
#[automatically_derived]
pub extern "C" fn process_introduction() {
    let result = cc_contract::__derived::exec_introduction();
    match result {
        Ok(()) => {
            let buf = ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!("execution successful"));
                res
            });
            ::borderless::log::print(::borderless::log::Level::Info, buf);
        }
        Err(e) => {
            let buf = ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(
                    format_args!("execution failed: {0:?}", e),
                );
                res
            });
            ::borderless::log::print(::borderless::log::Level::Error, buf);
        }
    }
}
#[no_mangle]
#[automatically_derived]
pub extern "C" fn process_revocation() {
    let result = cc_contract::__derived::exec_revocation();
    match result {
        Ok(()) => {
            let buf = ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!("execution successful"));
                res
            });
            ::borderless::log::print(::borderless::log::Level::Info, buf);
        }
        Err(e) => {
            let buf = ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(
                    format_args!("execution failed: {0:?}", e),
                );
                res
            });
            ::borderless::log::print(::borderless::log::Level::Error, buf);
        }
    }
}
#[no_mangle]
#[automatically_derived]
pub extern "C" fn http_get_state() {
    let result = cc_contract::__derived::exec_get_state();
    if let Err(e) = result {
        {
            let buf = ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(
                    format_args!("http-get failed: {0:?}", e),
                );
                res
            });
            ::borderless::log::print(::borderless::log::Level::Error, buf);
        };
    }
}
#[no_mangle]
#[automatically_derived]
pub extern "C" fn http_post_action() {
    let result = cc_contract::__derived::exec_post_action();
    if let Err(e) = result {
        {
            let buf = ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(
                    format_args!("http-post failed: {0:?}", e),
                );
                res
            });
            ::borderless::log::print(::borderless::log::Level::Error, buf);
        };
    }
}
#[no_mangle]
#[automatically_derived]
pub extern "C" fn get_symbols() {
    let result = cc_contract::__derived::get_symbols();
    if let Err(e) = result {
        {
            let buf = ::alloc::__export::must_use({
                let res = ::alloc::fmt::format(
                    format_args!("get-symbols failed: {0:?}", e),
                );
                res
            });
            ::borderless::log::print(::borderless::log::Level::Error, buf);
        };
    }
}
