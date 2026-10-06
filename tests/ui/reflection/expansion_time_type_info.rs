//@ compile-flags: --crate-type=lib -Znext-solver

#![feature(ettsd)]
#![no_std]

use core::expansion_time_type_system_demo;

expansion_time_type_system_demo!(core::primitive::u32);
//~^ ERROR: has eq impl
