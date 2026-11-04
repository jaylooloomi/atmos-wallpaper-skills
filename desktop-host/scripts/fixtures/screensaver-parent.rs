// An owned preview parent for native integration tests; never finds or controls user windows.
use std::{ptr::{null,null_mut},io::{self,Read}};
type H= *mut std::ffi::c_void;
#[repr(C)]struct Point{x:i32,y:i32}
#[repr(C)]struct Msg{window:H,message:u32,wparam:usize,lparam:isize,time:u32,point:Point,private:u32}
#[repr(C)]struct Class{style:u32,proc:Option<unsafe extern "system" fn(H,u32,usize,isize)->isize>,class_extra:i32,window_extra:i32,instance:H,icon:H,cursor:H,background:H,menu:*const u16,name:*const u16}
#[link(name="user32")]extern "system"{fn RegisterClassW(class:*const Class)->u16;fn CreateWindowExW(ex:u32,class:*const u16,title:*const u16,style:u32,x:i32,y:i32,width:i32,height:i32,parent:H,menu:H,instance:H,param:H)->H;fn DefWindowProcW(h:H,m:u32,w:usize,l:isize)->isize;fn PostQuitMessage(code:i32);fn PostMessageW(h:H,m:u32,w:usize,l:isize)->i32;fn GetMessageW(msg:*mut Msg,h:H,min:u32,max:u32)->i32;fn TranslateMessage(msg:*const Msg)->i32;fn DispatchMessageW(msg:*const Msg)->isize;}
#[link(name="kernel32")]extern "system"{fn GetModuleHandleW(name:*const u16)->H;}
fn wide(s:&str)->Vec<u16>{s.encode_utf16().chain([0]).collect()}
unsafe extern "system" fn procedure(h:H,m:u32,w:usize,l:isize)->isize{if m==2{PostQuitMessage(0);return 0}DefWindowProcW(h,m,w,l)}
fn main(){unsafe{let instance=GetModuleHandleW(null());let name=wide(&format!("AtmosOwnedScreensaverFixture{}",std::process::id()));let class=Class{style:0,proc:Some(procedure),class_extra:0,window_extra:0,instance,icon:null_mut(),cursor:null_mut(),background:null_mut(),menu:null(),name:name.as_ptr()};assert_ne!(RegisterClassW(&class),0);let parent=CreateWindowExW(0,name.as_ptr(),wide("Atmos owned preview fixture").as_ptr(),0x80000000,0,0,800,600,null_mut(),null_mut(),instance,null_mut());assert!(!parent.is_null());println!("{}",parent as isize);let handle=parent as isize;std::thread::spawn(move||{let mut input=String::new();let _=io::stdin().read_to_string(&mut input);unsafe{PostMessageW(handle as H,0x0010,0,0);}});let mut message:Msg=std::mem::zeroed();while GetMessageW(&mut message,null_mut(),0,0)>0{TranslateMessage(&message);DispatchMessageW(&message);}}}
