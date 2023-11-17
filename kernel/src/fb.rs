use core::ops::{BitXor, Div, Mul};
use core::ptr::slice_from_raw_parts;
use kernel_efi::gop::{PixelFormat,PixelBitmask};

///PS is PixelSize
pub struct FB<'a,'b,const PS:usize>{
	pub(crate) args:kernel_efi::Args<'a,'b>,
	pub(crate) ph:usize,
}

impl <'a,'b,const PS:usize> FB<'a,'b,PS>{
	fn get_bitmask(&self)->PixelBitmask{
		match self.args.gop.mode.info().pixel_format(){
			PixelFormat::Rgb => {PixelBitmask{
				reserved:0xFF00_0000,
				blue:0x00FF_0000,
				green:0x0000_FF00,
				red:0x0000_00FF,
			}}
			PixelFormat::Bgr => {PixelBitmask{
				reserved:0xFF00_0000,
				red:0x00FF_0000,
				green:0x0000_FF00,
				blue:0x0000_00FF,
			}}
			//Safety:
			// pixel_bitmask only returns some, if we have a PixelFormat::Bitmask.
			//  We are right now in exactly that case.
			PixelFormat::Bitmask => unsafe{core::hint::unreachable_unchecked()}
			//Safety:
			//in uefi-bin we filter out all BltOnly GOP output modes.
			PixelFormat::BltOnly => {unsafe{core::hint::unreachable_unchecked()}}
		}
	}
}
impl <'a,'b> FB<'a,'b, { core::mem::size_of::<u32>() }>{
	#[inline]
	fn get_pixel_value(&self,red:u8,green:u8,blue:u8)->u32{
		let format=self.args.gop.mode.info().pixel_format();
		#[cfg(feature = "core_intrinsics")]
		core::intrinsics::likely(format==PixelFormat::Bgr);
		if format==PixelFormat::Bgr{
			(blue as u32)|(green as u32)<<8|(red as u32)<<16
		} else if format==PixelFormat::Rgb{
			(red as u32)|(green as u32)<<8|(blue as u32)<<16
		} else{
			//Safety:
			//in uefi-bin we filter out all BltOnly and Bitmap GOP output modes.
			unsafe{core::hint::unreachable_unchecked()};
		}
	}
}

///PS = PixelSize in bytes
impl<'a,'b,const PS:usize> FB<'a,'b,PS>{
	fn render_char(&mut self,c:char){
		let font = &self.args.font;
		if let Some(g) = font.glyph_index(c){
			font.glyph_raster_image(g,font.units_per_em());
			if let Some(width) = font.glyph_hor_advance(g){
			
			}
			panic!();
		}
		panic!();
	}
	
	fn newline(&mut self){
		let (_,y) = self.args.gop.mode.info().resolution();
		let xs = self.args.gop.mode.info().stride();
		//Safety:
		//This is safe, because the framebuffer should be y*xs big.
		let fb_np=unsafe{self.args.gop.fb.as_mut_ptr().add(xs*self.ph)};
		let len = (y-self.ph)*xs*PS + self.args.font.vertical_line_gap().unwrap_or(0) as usize;
		let fb = &mut self.args.gop.fb;
		
		for i in 0..len{
			unsafe{
				core::ptr::write_volatile(fb.as_mut_ptr().add(i),core::ptr::read_volatile(fb_np.add(i)));
			}
		}
		for i in len..fb.size(){
			unsafe{
				core::ptr::write_volatile(fb.as_mut_ptr().add(i),0);
			}
		}
	}
    fn render_line(&mut self,s:&str){
	    for i in s.chars(){
		    self.render_char(i);
	    }
	    self.newline();
    }
}