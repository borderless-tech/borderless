#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
pub mod flipper {
    use borderless::{Result, *};
    use collections::lazyvec::LazyVec;
    use events::ActionOutput;
    use serde::{Deserialize, Serialize};
    pub struct History {
        switch: bool,
        counter: u32,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for History {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "History",
                "switch",
                &self.switch,
                "counter",
                &&self.counter,
            )
        }
    }
    #[automatically_derived]
    impl ::core::clone::Clone for History {
        #[inline]
        fn clone(&self) -> History {
            History {
                switch: ::core::clone::Clone::clone(&self.switch),
                counter: ::core::clone::Clone::clone(&self.counter),
            }
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for History {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for History {
        #[inline]
        fn eq(&self, other: &History) -> bool {
            self.switch == other.switch && self.counter == other.counter
        }
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
        impl _serde::Serialize for History {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "History",
                    false as usize + 1 + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "switch",
                    &self.switch,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "counter",
                    &self.counter,
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
        impl<'de> _serde::Deserialize<'de> for History {
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
                    __field1,
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
                            1u64 => _serde::__private::Ok(__Field::__field1),
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
                            "switch" => _serde::__private::Ok(__Field::__field0),
                            "counter" => _serde::__private::Ok(__Field::__field1),
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
                            b"switch" => _serde::__private::Ok(__Field::__field0),
                            b"counter" => _serde::__private::Ok(__Field::__field1),
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
                    marker: _serde::__private::PhantomData<History>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = History;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct History",
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
                            bool,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct History with 2 elements",
                                    ),
                                );
                            }
                        };
                        let __field1 = match _serde::de::SeqAccess::next_element::<
                            u32,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        1usize,
                                        &"struct History with 2 elements",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(History {
                            switch: __field0,
                            counter: __field1,
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
                        let mut __field0: _serde::__private::Option<bool> = _serde::__private::None;
                        let mut __field1: _serde::__private::Option<u32> = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field("switch"),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<bool>(&mut __map)?,
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private::Option::is_some(&__field1) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "counter",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::__private::Some(
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
                                _serde::__private::de::missing_field("switch")?
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private::Some(__field1) => __field1,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("counter")?
                            }
                        };
                        _serde::__private::Ok(History {
                            switch: __field0,
                            counter: __field1,
                        })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &["switch", "counter"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "History",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<History>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    pub struct Flipper {
        switch: bool,
        counter: u32,
        history: LazyVec<History>,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Flipper {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field3_finish(
                f,
                "Flipper",
                "switch",
                &self.switch,
                "counter",
                &self.counter,
                "history",
                &&self.history,
            )
        }
    }
    #[doc(hidden)]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate borderless as _borderless;
        #[doc(hidden)]
        #[automatically_derived]
        const fn __check_storeable<T: _borderless::__private::storage_traits::Storeable>() {}
        __check_storeable::<bool>();
        __check_storeable::<u32>();
        __check_storeable::<LazyVec<History>>();
        #[doc(hidden)]
        #[automatically_derived]
        const SYMBOLS: &[(&str, u64)] = &[
            ("switch", 10383125381033619138u64),
            ("counter", 13490770986381187908u64),
            ("history", 14474502600916996943u64),
        ];
        #[automatically_derived]
        impl _borderless::__private::storage_traits::State for Flipper {
            fn load() -> _borderless::Result<Self> {
                let switch = <bool as ::borderless::__private::storage_traits::Storeable>::decode(
                    10383125381033619138u64,
                );
                let counter = <u32 as ::borderless::__private::storage_traits::Storeable>::decode(
                    13490770986381187908u64,
                );
                let history = <LazyVec<
                    History,
                > as ::borderless::__private::storage_traits::Storeable>::decode(
                    14474502600916996943u64,
                );
                Ok(Self { switch, counter, history })
            }
            fn init(
                mut value: _borderless::serialize::Value,
            ) -> _borderless::Result<Self> {
                use _borderless::Context;
                let base_value = value
                    .get_mut("switch")
                    .take()
                    .context("failed to read parse field 'switch'")?;
                let switch = <bool as ::borderless::__private::storage_traits::Storeable>::parse_value(
                        base_value.clone(),
                        10383125381033619138u64,
                    )
                    .context("failed to read parse field 'switch'")?;
                let base_value = value
                    .get_mut("counter")
                    .take()
                    .context("failed to read parse field 'counter'")?;
                let counter = <u32 as ::borderless::__private::storage_traits::Storeable>::parse_value(
                        base_value.clone(),
                        13490770986381187908u64,
                    )
                    .context("failed to read parse field 'counter'")?;
                let base_value = value
                    .get_mut("history")
                    .take()
                    .context("failed to read parse field 'history'")?;
                let history = <LazyVec<
                    History,
                > as ::borderless::__private::storage_traits::Storeable>::parse_value(
                        base_value.clone(),
                        14474502600916996943u64,
                    )
                    .context("failed to read parse field 'history'")?;
                Ok(Self { switch, counter, history })
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
                    let value = <bool as ::borderless::__private::storage_traits::ToPayload>::to_payload(
                            &state.switch,
                            "",
                        )?
                        .context("failed to read parse field 'switch'")?;
                    buf.push('"');
                    buf.push_str("switch");
                    buf.push('"');
                    buf.push(':');
                    buf.push_str(&value);
                    buf.push(',');
                    let value = <u32 as ::borderless::__private::storage_traits::ToPayload>::to_payload(
                            &state.counter,
                            "",
                        )?
                        .context("failed to read parse field 'counter'")?;
                    buf.push('"');
                    buf.push_str("counter");
                    buf.push('"');
                    buf.push(':');
                    buf.push_str(&value);
                    buf.push(',');
                    let value = <LazyVec<
                        History,
                    > as ::borderless::__private::storage_traits::ToPayload>::to_payload(
                            &state.history,
                            "",
                        )?
                        .context("failed to read parse field 'history'")?;
                    buf.push('"');
                    buf.push_str("history");
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
                    "switch" => {
                        let value = <bool as ::borderless::__private::storage_traits::Storeable>::decode(
                            10383125381033619138u64,
                        );
                        <bool as ::borderless::__private::storage_traits::ToPayload>::to_payload(
                            &value,
                            suffix,
                        )
                    }
                    "counter" => {
                        let value = <u32 as ::borderless::__private::storage_traits::Storeable>::decode(
                            13490770986381187908u64,
                        );
                        <u32 as ::borderless::__private::storage_traits::ToPayload>::to_payload(
                            &value,
                            suffix,
                        )
                    }
                    "history" => {
                        let value = <LazyVec<
                            History,
                        > as ::borderless::__private::storage_traits::Storeable>::decode(
                            14474502600916996943u64,
                        );
                        <LazyVec<
                            History,
                        > as ::borderless::__private::storage_traits::ToPayload>::to_payload(
                            &value,
                            suffix,
                        )
                    }
                    _ => Ok(None),
                }
            }
            fn commit(self) {
                <bool as ::borderless::__private::storage_traits::Storeable>::commit(
                    self.switch,
                    10383125381033619138u64,
                );
                <u32 as ::borderless::__private::storage_traits::Storeable>::commit(
                    self.counter,
                    13490770986381187908u64,
                );
                <LazyVec<
                    History,
                > as ::borderless::__private::storage_traits::Storeable>::commit(
                    self.history,
                    14474502600916996943u64,
                );
            }
            fn symbols() -> &'static [(&'static str, u64)] {
                SYMBOLS
            }
        }
    };
    use self::actions::Actions;
    pub enum Other {
        Flipper(Actions),
    }
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
        __check_into_action::<Actions>();
        #[automatically_derived]
        impl _borderless::NamedSink for Other {
            fn into_action(self) -> (&'static str, _borderless::events::CallAction) {
                match self {
                    Other::Flipper(inner) => {
                        match inner.try_into() {
                            Ok(a) => ("Flipper", a),
                            Err(e) => {
                                {
                                    let buf = ::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(
                                            format_args!(
                                                "critical error while converting \'{0}\' of sink \'{1}\' into an action: {2}",
                                                "Actions",
                                                "Flipper",
                                                e,
                                            ),
                                        );
                                        res
                                    });
                                    ::borderless::log::print(
                                        ::borderless::log::Level::Error,
                                        buf,
                                    );
                                };
                                _borderless::__private::abort();
                            }
                        }
                    }
                }
            }
        }
    };
    impl Flipper {
        fn flip_switch(&mut self) -> ActionOutput {
            self.set_switch(!self.switch);
            let mut out = ActionOutput::default();
            out.add_event(Other::Flipper(Actions::FlipSwitch {}));
            out
        }
        fn set_switch(&mut self, switch: bool) {
            self.history
                .push(History {
                    switch: self.switch,
                    counter: self.counter,
                });
            self.counter += 1;
            self.switch = switch;
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
            ("flip_switch", 75999492u32),
            ("set_switch", 3376265057u32),
        ];
        #[doc(hidden)]
        #[automatically_derived]
        pub(crate) struct __FlipSwitchArgs {}
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
            impl _serde::Serialize for __FlipSwitchArgs {
                fn serialize<__S>(
                    &self,
                    __serializer: __S,
                ) -> _serde::__private::Result<__S::Ok, __S::Error>
                where
                    __S: _serde::Serializer,
                {
                    let __serde_state = _serde::Serializer::serialize_struct(
                        __serializer,
                        "__FlipSwitchArgs",
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
            impl<'de> _serde::Deserialize<'de> for __FlipSwitchArgs {
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
                        marker: _serde::__private::PhantomData<__FlipSwitchArgs>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    #[automatically_derived]
                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = __FlipSwitchArgs;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "struct __FlipSwitchArgs",
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
                            _serde::__private::Ok(__FlipSwitchArgs {})
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
                            _serde::__private::Ok(__FlipSwitchArgs {})
                        }
                    }
                    #[doc(hidden)]
                    const FIELDS: &'static [&'static str] = &[];
                    _serde::Deserializer::deserialize_struct(
                        __deserializer,
                        "__FlipSwitchArgs",
                        FIELDS,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<__FlipSwitchArgs>,
                            lifetime: _serde::__private::PhantomData,
                        },
                    )
                }
            }
        };
        #[doc(hidden)]
        #[automatically_derived]
        pub(crate) struct __SetSwitchArgs {
            pub(crate) switch: bool,
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
            impl _serde::Serialize for __SetSwitchArgs {
                fn serialize<__S>(
                    &self,
                    __serializer: __S,
                ) -> _serde::__private::Result<__S::Ok, __S::Error>
                where
                    __S: _serde::Serializer,
                {
                    let mut __serde_state = _serde::Serializer::serialize_struct(
                        __serializer,
                        "__SetSwitchArgs",
                        false as usize + 1,
                    )?;
                    _serde::ser::SerializeStruct::serialize_field(
                        &mut __serde_state,
                        "switch",
                        &self.switch,
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
            impl<'de> _serde::Deserialize<'de> for __SetSwitchArgs {
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
                                "switch" => _serde::__private::Ok(__Field::__field0),
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
                                b"switch" => _serde::__private::Ok(__Field::__field0),
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
                        marker: _serde::__private::PhantomData<__SetSwitchArgs>,
                        lifetime: _serde::__private::PhantomData<&'de ()>,
                    }
                    #[automatically_derived]
                    impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                        type Value = __SetSwitchArgs;
                        fn expecting(
                            &self,
                            __formatter: &mut _serde::__private::Formatter,
                        ) -> _serde::__private::fmt::Result {
                            _serde::__private::Formatter::write_str(
                                __formatter,
                                "struct __SetSwitchArgs",
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
                                bool,
                            >(&mut __seq)? {
                                _serde::__private::Some(__value) => __value,
                                _serde::__private::None => {
                                    return _serde::__private::Err(
                                        _serde::de::Error::invalid_length(
                                            0usize,
                                            &"struct __SetSwitchArgs with 1 element",
                                        ),
                                    );
                                }
                            };
                            _serde::__private::Ok(__SetSwitchArgs {
                                switch: __field0,
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
                            let mut __field0: _serde::__private::Option<bool> = _serde::__private::None;
                            while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                                __Field,
                            >(&mut __map)? {
                                match __key {
                                    __Field::__field0 => {
                                        if _serde::__private::Option::is_some(&__field0) {
                                            return _serde::__private::Err(
                                                <__A::Error as _serde::de::Error>::duplicate_field("switch"),
                                            );
                                        }
                                        __field0 = _serde::__private::Some(
                                            _serde::de::MapAccess::next_value::<bool>(&mut __map)?,
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
                                    _serde::__private::de::missing_field("switch")?
                                }
                            };
                            _serde::__private::Ok(__SetSwitchArgs {
                                switch: __field0,
                            })
                        }
                    }
                    #[doc(hidden)]
                    const FIELDS: &'static [&'static str] = &["switch"];
                    _serde::Deserializer::deserialize_struct(
                        __deserializer,
                        "__SetSwitchArgs",
                        FIELDS,
                        __Visitor {
                            marker: _serde::__private::PhantomData::<__SetSwitchArgs>,
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
                                "flip_switch" => {
                                    let _args: __derived::__FlipSwitchArgs = ::borderless::serialize::from_value(
                                        action.params.clone(),
                                    )?;
                                }
                                "set_switch" => {
                                    let writer_roles = ::borderless::contracts::env::writer_roles();
                                    if !writer_roles
                                        .iter()
                                        .any(|role| role.eq_ignore_ascii_case("Flipper"))
                                    {
                                        let writer = ::borderless::contracts::env::writer();
                                        return Err(
                                            ::anyhow::Error::msg(
                                                ::alloc::__export::must_use({
                                                    let res = ::alloc::fmt::format(
                                                        format_args!(
                                                            "writer {0} has no access to action \'{1}\'",
                                                            writer,
                                                            "set_switch",
                                                        ),
                                                    );
                                                    res
                                                }),
                                            ),
                                        );
                                    }
                                    let _args: __derived::__SetSwitchArgs = ::borderless::serialize::from_value(
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
                                75999492u32 => {
                                    let _args: __derived::__FlipSwitchArgs = ::borderless::serialize::from_value(
                                        action.params.clone(),
                                    )?;
                                }
                                3376265057u32 => {
                                    let writer_roles = ::borderless::contracts::env::writer_roles();
                                    if !writer_roles
                                        .iter()
                                        .any(|role| role.eq_ignore_ascii_case("Flipper"))
                                    {
                                        let writer = ::borderless::contracts::env::writer();
                                        return Err(
                                            ::anyhow::Error::msg(
                                                ::alloc::__export::must_use({
                                                    let res = ::alloc::fmt::format(
                                                        format_args!(
                                                            "writer {0} has no access to action \'{1}\'",
                                                            writer,
                                                            "set_switch",
                                                        ),
                                                    );
                                                    res
                                                }),
                                            ),
                                        );
                                    }
                                    let _args: __derived::__SetSwitchArgs = ::borderless::serialize::from_value(
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
                "flip_switch" => {
                    let _args: __derived::__FlipSwitchArgs = ::borderless::serialize::from_slice(
                        &payload,
                    )?;
                    let value = ::borderless::serialize::to_value(&_args)?;
                    Ok(CallAction::by_method("flip_switch", value))
                }
                "set_switch" => {
                    let writer_roles = ::borderless::contracts::env::writer_roles();
                    if !writer_roles
                        .iter()
                        .any(|role| role.eq_ignore_ascii_case("Flipper"))
                    {
                        let writer = ::borderless::contracts::env::writer();
                        return Err(
                            ::anyhow::Error::msg(
                                ::alloc::__export::must_use({
                                    let res = ::alloc::fmt::format(
                                        format_args!(
                                            "writer {0} has no access to action \'{1}\'",
                                            writer,
                                            "set_switch",
                                        ),
                                    );
                                    res
                                }),
                            ),
                        );
                    }
                    let _args: __derived::__SetSwitchArgs = ::borderless::serialize::from_slice(
                        &payload,
                    )?;
                    let value = ::borderless::serialize::to_value(&_args)?;
                    Ok(CallAction::by_method("set_switch", value))
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
                <Flipper as ::borderless::__private::storage_traits::State>::symbols(),
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
            let mut state = <Flipper as ::borderless::__private::storage_traits::State>::load()?;
            let _match_result = match &action.method {
                MethodOrId::ByName { method } => {
                    match method.as_str() {
                        "flip_switch" => {
                            let result = Flipper::flip_switch(&mut state);
                            <ActionOutput as ::borderless::events::ActionOutEvent>::convert_out_events(
                                result,
                            )
                        }
                        "set_switch" => {
                            let writer_roles = ::borderless::contracts::env::writer_roles();
                            if !writer_roles
                                .iter()
                                .any(|role| role.eq_ignore_ascii_case("Flipper"))
                            {
                                let writer = ::borderless::contracts::env::writer();
                                return Err(
                                    ::anyhow::Error::msg(
                                        ::alloc::__export::must_use({
                                            let res = ::alloc::fmt::format(
                                                format_args!(
                                                    "writer {0} has no access to action \'{1}\'",
                                                    writer,
                                                    "set_switch",
                                                ),
                                            );
                                            res
                                        }),
                                    ),
                                );
                            }
                            let args: __derived::__SetSwitchArgs = ::borderless::serialize::from_value(
                                action.params,
                            )?;
                            let result = Flipper::set_switch(&mut state, args.switch);
                            <() as ::borderless::events::ActionOutEvent>::convert_out_events(
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
                        75999492u32 => {
                            let result = Flipper::flip_switch(&mut state);
                            <ActionOutput as ::borderless::events::ActionOutEvent>::convert_out_events(
                                result,
                            )
                        }
                        3376265057u32 => {
                            let writer_roles = ::borderless::contracts::env::writer_roles();
                            if !writer_roles
                                .iter()
                                .any(|role| role.eq_ignore_ascii_case("Flipper"))
                            {
                                let writer = ::borderless::contracts::env::writer();
                                return Err(
                                    ::anyhow::Error::msg(
                                        ::alloc::__export::must_use({
                                            let res = ::alloc::fmt::format(
                                                format_args!(
                                                    "writer {0} has no access to action \'{1}\'",
                                                    writer,
                                                    "set_switch",
                                                ),
                                            );
                                            res
                                        }),
                                    ),
                                );
                            }
                            let args: __derived::__SetSwitchArgs = ::borderless::serialize::from_value(
                                action.params,
                            )?;
                            let result = Flipper::set_switch(&mut state, args.switch);
                            <() as ::borderless::events::ActionOutEvent>::convert_out_events(
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
            <Flipper as ::borderless::__private::storage_traits::State>::commit(state);
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
            let state = <Flipper as ::borderless::__private::storage_traits::State>::init(
                introduction.initial_state,
            )?;
            <Flipper as ::borderless::__private::storage_traits::State>::commit(state);
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
            let result = <Flipper as ::borderless::__private::storage_traits::State>::http_get(
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
            FlipSwitch {},
            SetSwitch { switch: bool },
        }
        #[automatically_derived]
        impl TryFrom<Actions> for ::borderless::events::CallAction {
            type Error = ::borderless::serialize::Error;
            fn try_from(
                value: Actions,
            ) -> ::std::result::Result<::borderless::events::CallAction, Self::Error> {
                let action = match value {
                    Actions::FlipSwitch {} => {
                        let args = __FlipSwitchArgs {};
                        let args_value = ::borderless::serialize::to_value(&args)?;
                        ::borderless::events::CallAction::by_method(
                            "flip_switch",
                            args_value,
                        )
                    }
                    Actions::SetSwitch { switch } => {
                        let args = __SetSwitchArgs { switch };
                        let args_value = ::borderless::serialize::to_value(&args)?;
                        ::borderless::events::CallAction::by_method(
                            "set_switch",
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
    let result = flipper::__derived::exec_txn();
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
    let result = flipper::__derived::exec_introduction();
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
    let result = flipper::__derived::exec_revocation();
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
    let result = flipper::__derived::exec_get_state();
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
    let result = flipper::__derived::exec_post_action();
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
    let result = flipper::__derived::get_symbols();
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
