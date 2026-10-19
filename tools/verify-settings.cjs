// 编译运行真实显血菜单与共用存档读写，只替换控件输入和文件系统。
const fs=require('fs'),path=require('path'),cp=require('child_process');
const root=path.resolve(__dirname,'..'),repo=path.dirname(root),out=path.join(root,'target/settings-check');
fs.mkdirSync(out,{recursive:true});
const rust=p=>JSON.stringify(p.replaceAll(String.fromCharCode(92),'/'));
const logic=fs.readFileSync(path.join(root,'src/lib.rs'),'utf8');
const reader=logic.slice(logic.indexOf('fn load_config()'),logic.indexOf('#[unsafe(no_mangle)]'));
const menu=logic.slice(logic.indexOf('#[forge::imgui_render]'),logic.indexOf('unsafe fn read_u32(')).replace('#[forge::imgui_render]','');
let mock=fs.readFileSync(path.join(__dirname,'settings-host.rs'),'utf8');
mock=mock.slice(0,mock.indexOf('fn click('))
 .replace('pub struct ImGuiSliderFlags; impl ImGuiSliderFlags { pub const None: Self = Self; }','pub struct ImGuiSliderFlags; impl ImGuiSliderFlags {pub const None:Self=Self;pub const AlwaysClamp:Self=Self;}')
 .replace('button: String, stack:', 'input:Option<String>,colors:BTreeMap<String,[f32;4]>,button:String,stack:')
 .replace('pub fn text(text: &str)',`pub fn color_edit4(label:&CStr,color:&mut[f32;4],flags:sys::imgui::ImGuiColorEditFlags)->bool {
    assert_ne!(flags.0 & sys::imgui::ImGuiColorEditFlags::DisplayHex.0,0);
    CONTROLS.with(|c|if let Some(v)=c.borrow_mut().colors.remove(label.to_str().unwrap()){*color=v;true}else{false})
 }
 pub fn input_text(_: &CStr,value:&mut[u8],_:sys::imgui::ImGuiInputTextFlags)->bool {
    CONTROLS.with(|c|if let Some(v)=c.borrow_mut().input.take(){value.fill(0);value[..v.len()].copy_from_slice(v.as_bytes());true}else{false})
 }
 pub fn text(text: &str)`);
const source=`#![allow(dead_code,unused_imports,non_upper_case_globals)]
extern crate self as forge;
#[path=${rust(path.join(root,'src/data.rs'))}]mod data;use data::*;
#[path=${rust(path.join(root,'shared/plugin_config.rs'))}]mod plugin_config;
use plugin_config::HEALTH as LOCATION;
use core::sync::atomic::Ordering;
use sys::imgui::{ImGuiInputTextFlags,ImGuiSliderFlags,ImGuiTreeNodeFlags};
${mock}
${reader}
${menu}
fn active()->Config{STATE.with(|s|s.config)}
fn saved()->Config{SAVED_CONFIG.with(|s|*s)}
fn disk()->Config{STORAGE.with(|s|Config::parse(std::str::from_utf8(&s.borrow().files[&saved_path()]).unwrap()).unwrap())}
fn click(label:&str){CONTROLS.with(|c|c.borrow_mut().button=label.into());draw_settings();}
fn ui_start(config:Config){STATE.with(|s|s.config=config);SAVED_CONFIG.with(|s|*s=config);set_font_input(&config);READY.store(true,Ordering::Release);ENABLED.store(config.enabled,Ordering::Relaxed);CONTROLS.with(|s|*s.borrow_mut()=Controls::default());SAVE_STATUS.store(0,Ordering::Relaxed);}
fn main(){
 assert_eq!(load_config(),Config::DEFAULT);assert_eq!(disk(),Config::DEFAULT);
 STORAGE.with(|s|assert_eq!(s.borrow().writes,1));load_config();STORAGE.with(|s|assert_eq!(s.borrow().writes,1));
 let config=Config::parse("[bar]\\nenabled=false").unwrap();save_config(&config).unwrap();ui_start(config);
 let before=STORAGE.with(|s|s.borrow().writes);draw_settings();assert_eq!(STORAGE.with(|s|s.borrow().writes),before);
 CONTROLS.with(|c|{let mut c=c.borrow_mut();
 c.bools.insert("Show selected monster health".into(),true);c.bools.insert("Show health in other areas".into(),false);
 c.bools.insert("Monster name".into(),false);c.bools.insert("Show HP percentage".into(),false);
 c.floats.insert("Bar X".into(),123.5);c.floats.insert("Information font size".into(),33.0);
 c.colors.insert("Bar fill color".into(),[0.0,0.5,1.0,0.5]);c.ints.insert("HP text: 0 inside / 1 above / 2 below".into(),2);});
 draw_settings();let edited=active();assert!(edited.enabled&&!edited.show_other_areas&&!edited.info.show_name&&!edited.show_percent);
 assert_eq!(edited.x,123.5);assert_eq!(edited.info.font_size,33.0);assert_eq!(edited.fill_color,[0,128,255,128]);assert_eq!(edited.text_position,TextPosition::Below);
 assert_ne!(edited,disk());assert_eq!(saved(),config);assert!(ENABLED.load(Ordering::Relaxed));
 click("Save INI");assert_eq!(saved(),edited);assert_eq!(disk(),edited);assert_eq!(load_config(),edited);
 STORAGE.with(|s|assert!(std::str::from_utf8(&s.borrow().files[&saved_path()]).unwrap().contains("fill_color = #0080FF80")));
 click("Defaults");assert_eq!(active(),Config::DEFAULT);assert_eq!(disk(),edited);click("Revert changes");assert_eq!(active(),edited);
 click("Defaults");STORAGE.with(|s|s.borrow_mut().write_error=true);click("Save INI");assert_eq!(SAVE_STATUS.load(Ordering::Relaxed),2);assert_eq!(saved(),edited);
 CONTROLS.with(|c|assert!(c.borrow().text.iter().any(|s|s.contains("Save failed"))));
 STORAGE.with(|s|s.borrow_mut().write_error=false);click("Revert changes");click("Save INI");assert_eq!(disk(),edited);
 for invalid in ["bad#font.ttf","../bad.ttf",""]{CONTROLS.with(|c|c.borrow_mut().input=Some(invalid.into()));draw_settings();
 let writes=STORAGE.with(|s|s.borrow().writes);click("Save INI");assert_eq!(SAVE_STATUS.load(Ordering::Relaxed),4);assert_eq!(STORAGE.with(|s|s.borrow().writes),writes);assert_eq!(saved(),edited);}
 CONTROLS.with(|c|c.borrow_mut().input=Some("思源黑体.ttf".into()));draw_settings();click("Save INI");assert_eq!(load_config().font_file.unwrap().filename(),"思源黑体.ttf");
 CONTROLS.with(|c|c.borrow_mut().floats.insert("Bar width".into(),f32::NAN));let width=active().width;draw_settings();assert_eq!(active().width,width);
 CONTROLS.with(|c|c.borrow_mut().floats.insert("Bar width".into(),9000.0));draw_settings();assert_eq!(active().width,1280.0);
 READY.store(false,Ordering::Release);let before=STORAGE.with(|s|s.borrow().writes);click("Save INI");assert_eq!(STORAGE.with(|s|s.borrow().writes),before);
 STORAGE.with(|s|{let s=s.borrow();assert_eq!(s.handles,0);assert_eq!(s.active_mounts,0);});
 println!("PASS: real health settings; HEX RGBA controls, live preview, save/reload same save-root INI, defaults/revert, invalid font, finite bounds, write failures, balanced mounts");
}
`;
const src=path.join(out,'settings-host.rs'),exe=path.join(out,'settings-host.exe');fs.writeFileSync(src,source);
cp.execFileSync(process.env.RUSTC||'rustc',['--edition=2024',src,'-o',exe],{stdio:'inherit'});
cp.execFileSync(exe,{stdio:'inherit'});
fs.writeFileSync(path.join(root,'target/settings-verification.json'),JSON.stringify({source:'actual src/lib.rs + data.rs + shared storage',location:'MHGU save root',hex_rgba:true,game_tested:false},null,2)+'\n');
