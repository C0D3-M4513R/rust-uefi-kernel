use ttf_parser::OutlineBuilder;

const loc:&str = "C:/Users/timon/Downloads/anonymos pro/Anonymous_Pro.ttf";

fn main() -> Result<(),()>{
    
    let ff = std::fs::read(loc).map_err(|_|())?;
    let f = ttf_parser::Face::from_slice(ff.as_slice(), 0).map_err(|_|())?;
    
    let gi= f.glyph_index('0').unwrap();
    let mut d = DEMO{};
    let o = f.outline_glyph(gi,&mut d);
    
    println!("Hello, world! {:#?}",o);
    Ok(())
}

struct DEMO{}

impl OutlineBuilder for DEMO{
    fn move_to(&mut self, x: f32, y: f32) {
        println!("M {} {}",x,y);
    }
    
    fn line_to(&mut self, x: f32, y: f32) {
        println!("L {} {}",x,y);
    }
    
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        println!("Q {} {} {} {}",x1,y1,x,y);
    }
    
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        println!("C {} {} {} {} {} {}",x1,y1,x2,y2,x,y);
        eprintln!("Curve!");
        todo!()
    }
    
    fn close(&mut self) {
        println!("Z");
    }
}