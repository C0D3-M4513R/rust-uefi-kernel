use uefi::{Error, Guid, Handle, Status};
use uefi::proto::console::gop::{GraphicsOutput, Mode, ModeInfo, PixelFormat};
use uefi::Result;
use uefi::table::boot::{OpenProtocolAttributes, OpenProtocolParams, SearchType};
use uefi::Identify;

pub fn get_best_gop_fb(handle:Handle)->Result<kernel_efi::GOP>{
	//if uefi_services::system_table().as_ptr().is_null(){panic!();}
	//SAFETY:
	// The SystemTable is passed to Main fn.
	// uefi_services stores a ref. of that SystemTable
	// uefi_services casts that ref ptr to a ptr only to pass it back. why?
	let st = unsafe{uefi_services::system_table().as_mut()};
	let mut gop_op = None;
	let handles = st.boot_services()
		.locate_handle_buffer(SearchType::ByProtocol(&GraphicsOutput::GUID))?;
	for h in &*handles {
		let gop_p_candidate = unsafe{st.boot_services().open_protocol::<GraphicsOutput>(
			OpenProtocolParams {
				handle: *h,
				agent: handle,
				controller: None,
			},
			OpenProtocolAttributes::Exclusive,
		)};
		match gop_p_candidate{
			Ok(v)=>{
				gop_op=Some(v);
				break;
			},
			Err(_)=>(),
		}
	}
	//todo: have a better way, to draw stuff
	let mut gop_p;
	match gop_op{
		None => return Err(Error::from(Status::NOT_FOUND)),
		Some(proto)=>{
			gop_p=proto;
		}
	}
	//SAFETY:
	// The protocol was opened in exclusive mode. UEFI should satisfy exclusive control.
	let mut gop = &mut *gop_p;
	{
		let i = gop.modes();
		let i = i.filter(|m|{let f = m.info().pixel_format(); f==PixelFormat::Bgr||f==PixelFormat::Rgb});
		let mut mi:Option<Mode>=None;
		for m in i{
			if let Some(mis)=&mi{
				let (rxn,ryn) = m.info().resolution();
				let (rx,ry) = mis.info().resolution();
				
				let same_or_better_res = rxn>=rx && ryn>=ry;
				let better_pf = (m.info().pixel_format() as u32) < (mis.info().pixel_format() as u32);
				let same_or_better_pf = (m.info().pixel_format() as u32) <= (mis.info().pixel_format() as u32);
				//There is a better resolution available
				if (ry!=ryn || rx!=rxn) && rxn>=rx && ryn>=ry {
					mi=Some(m);
					//There is an easier pixel format to work in
				}else if better_pf && same_or_better_res {
					mi=Some(m);
					//we can waste less space.
				}else if m.info().stride() < mis.info().stride() && same_or_better_pf && same_or_better_res {
					mi=Some(m);
				}else{
					continue;
				}
			} else {
				mi=Some(m);
			}
		}
		if let Some(m)=mi{
			gop.set_mode(&m)?;
			let mut fb = gop.frame_buffer();
			return Ok(kernel_efi::GOP{ fb: kernel_efi::FB {
				base: fb.as_mut_ptr(),
				size: fb.size(),
			}, mode: m })
		}
	}
	Err(uefi::Error::new(Status::DEVICE_ERROR,()))
}