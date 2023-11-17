/// Bitmask used to indicate which bits of a pixel represent a given color.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub struct PixelBitmask {
	/// The bits indicating the red channel.
	pub red: u32,
	/// The bits indicating the green channel.
	pub green: u32,
	/// The bits indicating the blue channel.
	pub blue: u32,
	/// The reserved bits, which are ignored by the video hardware.
	pub reserved: u32,
}