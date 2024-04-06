// SPDX-License-Identifier: GPL-2.0

//! Rust character device sample.

use core::default::Default;
use core::result::Result::Err;

use kernel::prelude::*;
use kernel::{chrdev, file, bindings, task};

module! {
    type: RustChrdev,
    name: "completion",
    author: "Rust for Linux Contributors",
    description: "Rust character device sample",
    license: "GPL",
}

struct CompletionStruct(bindings::completion);

unsafe impl Send for CompletionStruct {}
unsafe impl Sync for CompletionStruct {}

static mut GLOBALMEM_COMP: Option<CompletionStruct> = None;

struct RustFile {
}

#[vtable]
impl file::Operations for RustFile {
    type Data = Box<Self>;

    fn open(_shared: &(), _file: &file::File) -> Result<Box<Self>> {
        pr_info!("open is invoked\n");
        Ok(
            Box::try_new(RustFile {
            })?
        )
    }

    fn write(_this: &Self,_file: &file::File,_reader: &mut impl kernel::io_buffer::IoBufferReader,_offset:u64,) -> Result<usize> {
        pr_info!("write is invoked\n");
        pr_info!("process {} awakening the readers...\n", task::Task::current().pid());
        unsafe {
            match &mut GLOBALMEM_COMP {
                Some(ref mut completion) => {
                    let ptr = &mut completion.0 as *mut bindings::completion;
                    bindings::complete(ptr);
                }
                None => {
                    pr_info!("None\n");
                }
            }
        }
        Ok(_reader.len())
    }

    fn read(_this: &Self,_file: &file::File,_writer: &mut impl kernel::io_buffer::IoBufferWriter,_offset:u64,) -> Result<usize> {
        pr_info!("read is invoked\n");
        pr_info!("process {} is going to sleep\n", task::Task::current().pid());
        unsafe {
            match &mut GLOBALMEM_COMP {
                Some(ref mut completion) => {
                    let ptr = &mut completion.0 as *mut bindings::completion;
                    bindings::wait_for_completion(ptr);
                }
                None => {
                    pr_info!("None\n");
                }
            }
        }

        pr_info!("awoken {}\n", task::Task::current().pid());
        Ok(0)
    }
}

struct RustChrdev {
    _dev: Pin<Box<chrdev::Registration<2>>>,
}

impl kernel::Module for RustChrdev {
    fn init(name: &'static CStr, module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust character device sample (init)\n");
        unsafe {GLOBALMEM_COMP = Some(CompletionStruct(bindings::completion::default()));}
        unsafe {
            match &mut GLOBALMEM_COMP {
                Some(ref mut completion) => {
                    let ptr = &mut completion.0 as *mut bindings::completion;
                    bindings::init_completion(ptr);
                }
                None => {
                    pr_info!("None\n");
                }
            }
        }

        let mut chrdev_reg = chrdev::Registration::new_pinned(name, 0, module)?;

        // Register the same kind of device twice, we're just demonstrating
        // that you can use multiple minors. There are two minors in this case
        // because its type is `chrdev::Registration<2>`
        chrdev_reg.as_mut().register::<RustFile>()?;
        chrdev_reg.as_mut().register::<RustFile>()?;
 
        Ok(RustChrdev { _dev: chrdev_reg })
    }
}

impl Drop for RustChrdev {
    fn drop(&mut self) {
        pr_info!("Rust character device sample (exit)\n");
    }
}
