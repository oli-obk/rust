//@ compile-flags: --crate-type=lib
//~^ ERROR: cycle detected

#![feature(ettsd)]
#![no_std]

use core::expansion_time_type_system_demo;

expansion_time_type_system_demo!(core::alloc::Layout);
