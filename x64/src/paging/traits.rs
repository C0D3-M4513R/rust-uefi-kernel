use core::cmp::Ordering;

#[derive(Copy, Clone,Debug)]
pub enum LevelEnum {
	Level5,
	Level4,
	Level3,
	Level2,
	Level1
}

///This trait represents all Page Levels that exist.
pub trait Level{
	fn get_level()->LevelEnum where Self: Sized;
}
///This trait represents all Page Levels, where it is appropriate, to have a Page Table
pub trait LevelTable:Level{
	type Down:Level;
	
}
pub enum Level1{}
impl Level for Level1{
	fn get_level() -> LevelEnum {
		LevelEnum::Level1
	}
}
pub enum Level2{}
impl Level for Level2{
	fn get_level() -> LevelEnum {
		LevelEnum::Level2
	}
}
impl LevelTable for Level2{
	type Down = Level1;
}
pub enum Level3{}
impl Level for Level3 {
	fn get_level() -> LevelEnum {
		LevelEnum::Level3
	}
}
impl LevelTable for Level3{
	type Down = Level2;
}
pub enum Level4{}
impl Level for Level4{
	fn get_level() -> LevelEnum {
		LevelEnum::Level4
	}
}
impl LevelTable for Level4{
	type Down = Level3;
}
pub enum Level5{}
impl Level for Level5{
	fn get_level() -> LevelEnum {
		LevelEnum::Level5
	}
}
impl LevelTable for Level5{
	type Down = Level4;
}

impl LevelEnum {
	///Constructs a Level enum from a number, if possible
	pub fn from_level(l:u8)->Option<Self>{
		match l {
			1=>Some(LevelEnum::Level1),
			2=>Some(LevelEnum::Level2),
			3=>Some(LevelEnum::Level3),
			4=>Some(LevelEnum::Level4),
			5=>Some(LevelEnum::Level5),
			_=>None,
		}
	}
	///Gets the Level as a number
	pub fn get_level(&self) ->u8{
		match self {
			LevelEnum::Level5 => 5,
			LevelEnum::Level4 => 4,
			LevelEnum::Level3 => 3,
			LevelEnum::Level2 => 2,
			LevelEnum::Level1 => 1,
		}
	}
	///gets the next lower level
	pub fn next_lower_level(&self)->Option<LevelEnum>{
		match self {
			LevelEnum::Level5 => Some(LevelEnum::Level4),
			LevelEnum::Level4 => Some(LevelEnum::Level3),
			LevelEnum::Level3 => Some(LevelEnum::Level2),
			LevelEnum::Level2 => Some(LevelEnum::Level1),
			LevelEnum::Level1 => None
		}
	}
	///Gets the addressed size, of a entry in a page table with that level, in byte
	fn get_size(&self) -> u64 {
		match self {
			LevelEnum::Level5 => 256*1024*1204*1024*1024,
			LevelEnum::Level4 => 512*1024*1024*1024,
			LevelEnum::Level3 => 1*1024*1024*1024,
			LevelEnum::Level2 => 2*1024*1024,
			LevelEnum::Level1 => 4*1024,
		}
	}
}
impl PartialEq for LevelEnum {
	fn eq(&self, other: &Self) -> bool {
		other.get_level()==self.get_level()
	}
}
impl Eq for LevelEnum {}

impl PartialOrd for LevelEnum {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.get_level().partial_cmp(&other.get_level())
	}
}
impl Ord for LevelEnum {
	fn cmp(&self, other: &Self) -> Ordering {
		self.get_level().cmp(&other.get_level())
	}
}
impl Default for LevelEnum {
	fn default() -> Self {
		LevelEnum::Level1
	}
}