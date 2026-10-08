//@ compile-flags: --crate-type=lib -Znext-solver

#![feature(ettsd)]
#![no_std]

use core::expansion_time_type_system_demo;

expansion_time_type_system_demo!(core::cmp::PartialEq, core::primitive::u32, core::primitive::u32);
//~^ ERROR: is implemented

expansion_time_type_system_demo!(core::cmp::PartialEq, core::alloc::Layout, core::alloc::Layout);
//~^ ERROR: is implemented

expansion_time_type_system_demo!(core::cmp::Eq, core::primitive::f32);
//~^ ERROR: is not implemented

expansion_time_type_system_demo!(core::cmp::Eq, core::primitive::u32);
//~^ ERROR: is implemented

expansion_time_type_system_demo!(core::fmt::Display, core::alloc::Layout);
//~^ ERROR: is not implemented
