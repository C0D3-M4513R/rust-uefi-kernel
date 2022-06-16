//From: https://github.com/borntyping/rust-simple_logger . Thank You!
//Copyright 2015-2021 Sam Clements
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
#![no_std]
extern crate log;
extern crate core;

#[cfg(feature = "time")]
use time::OffsetDateTime;

use core::fmt::Write;
use core::mem::MaybeUninit;
use log::{Level, LevelFilter, Log, Metadata, Record};
use uefi::proto::console::text::{Output,Color};

static mut OUTPUT:MaybeUninit<&mut Output>=MaybeUninit::uninit();

/// Implements [`Log`] and a set of simple builder methods for configuration.
///
/// Use the various "builder" methods on this struct to configure the logger,
/// then call [`init`] to configure the [`log`] crate.
pub struct Logger{
	/// The default logging level
	default_level: LevelFilter,
	/// The specific logging level for each module
	///
	/// This is used to override the default value for some specific modules.
	/// After initialization, the vector is sorted so that the first (prefix) match
	/// directly gives us the desired log level.
	// module_levels: Vec<(String,LevelFilter)>,
	
	/// Whether to use color output or not.
	///
	/// This field is only available if the `color` feature is enabled.
	colors: bool,
}

impl Logger {
	pub fn set_output(o:&'static mut Output) {
		unsafe{
			OUTPUT=MaybeUninit::new(o);
		}
	}
	/// Initializes the global logger with a SimpleLogger instance with
	/// default log level set to `Level::Trace`.
	///
	///
	/// [`init`]: #method.init
	#[must_use = "You must call init() to begin logging"]
	pub const fn new() -> Self {
		Self {
			default_level: LevelFilter::Trace,
			// module_levels: Vec::new(),
			colors: true,
		}
	}
	
	/// Set the 'default' log level.
	///
	/// You can override the default level for specific modules and their sub-modules using [`with_module_level`]
	///
	/// [`with_module_level`]: #method.with_module_level
	#[must_use = "You must call init() to begin logging"]
	pub const fn with_level(mut self, level: LevelFilter) -> Self {
		self.default_level = level;
		self
	}
	
	/// Override the log level for some specific modules.
	///
	/// This sets the log level of a specific module and all its sub-modules.
	/// When both the level for a parent module as well as a child module are set,
	/// the more specific value is taken. If the log level for the same module is
	/// specified twice, the resulting log level is implementation defined.
	///
	/// # Examples
	///
	/// Silence an overly verbose crate:
	///
	/// ```no_run
	/// use simple_logger::SimpleLogger;
	/// use log::LevelFilter;
	///
	/// SimpleLogger::new().with_module_level("chatty_dependency", LevelFilter::Warn).init().unwrap();
	/// ```
	///
	/// Disable logging for all dependencies:
	///
	/// ```no_run
	/// use simple_logger::SimpleLogger;
	/// use log::LevelFilter;
	///
	/// SimpleLogger::new()
	///     .with_level(LevelFilter::Off)
	///     .with_module_level("my_crate", LevelFilter::Info)
	///     .init()
	///     .unwrap();
	/// ```
	// #[must_use = "You must call init() to begin logging"]
	// pub fn with_module_level(mut self, target: &str, level: LevelFilter) -> Self {
	// 	self.module_levels.push((target.to_string(), level));
	//
	// 	/* Normally this is only called in `init` to avoid redundancy, but we can't initialize the logger in tests */
	// 	#[cfg(test)]
	// 	self.module_levels
	// 		.sort_by_key(|(name, _level)| name.len().wrapping_neg());
	//
	// 	self
	// }

	
	/// Control whether messages are colored or not.
	///
	/// This method is only available if the `colored` feature is enabled.
	#[must_use = "You must call init() to begin logging"]
	pub const fn with_colors(mut self, colors: bool) -> Self {
		self.colors = colors;
		self
	}
	
	/// 'Init' the actual logger, instantiate it and configure it,
	/// this method MUST be called in order for the logger to be effective.
	pub fn init(&mut self){
		/* Sort all module levels from most specific to least specific. The length of the module
		 * name is used instead of its actual depth to avoid module name parsing.
		 */
		// self.module_levels
		// 	.sort_by_key(|(name, _level)| name.len().wrapping_neg());
		// let max_level = self
		// 	.module_levels
		// 	.iter()
		// 	.map(|(_name, level)| level)
		// 	.copied()
		// 	.max();
		// let max_level = max_level
		// 	.map(|lvl| lvl.max(self.default_level))
		// 	.unwrap_or(self.default_level);
		log::set_max_level(self.default_level);
	}
}

impl Log for Logger {
	fn enabled(&self, metadata: &Metadata) -> bool {
		&metadata.level().to_level_filter()
			<= /*self
			.module_levels
			.iter()
			/* At this point the Vec is already sorted so that we can simply take
			 * the first match
			 */
			.find(|(name, _level)| metadata.target().starts_with(name))
			.map(|(_name, level)| level)
			.unwrap_or(&self.default_level)
			*/
		&self.default_level
	}
	
	fn log(&self, record: &Record) {
		if self.enabled(record.metadata()) {
			
			let target = if !record.target().is_empty() {
				record.target()
			} else {
				record.module_path().unwrap_or_default()
			};
			
			if self.colors {
				let _=match record.level() {
					Level::Error => unsafe{OUTPUT.assume_init_mut().set_color(Color::Red,Color::Black)},
					Level::Warn => unsafe{OUTPUT.assume_init_mut().set_color(Color::Yellow,Color::Black)},
					Level::Info => unsafe{OUTPUT.assume_init_mut().set_color(Color::Cyan,Color::Black)},
					Level::Debug => unsafe{OUTPUT.assume_init_mut().set_color(Color::Magenta,Color::Black)},
					Level::Trace => unsafe{OUTPUT.assume_init_mut().set_color(Color::LightGray,Color::Black)},
				}.err();
			}
			unsafe{OUTPUT.assume_init_mut().write_str(record.level().as_str())}.err();
			if self.colors{
				unsafe{OUTPUT.assume_init_mut().set_color(Color::White,Color::Black).err();}
			}
			let o=unsafe{OUTPUT.assume_init_mut()};
			o.write_char('[').ok();
			o.write_str(target).ok();
			o.write_str("] ").ok();
			core::fmt::write(o,*record.args()).ok();
			// match record.args().as_str() {
			// 	Some(r)=>o.write_str(r).ok(),
			// 	None=>None,
			// };
			o.write_str("\r\n").ok();
		}
	}
	
	fn flush(&self) {}
}