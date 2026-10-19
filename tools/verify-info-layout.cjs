// 用真实 Dear ImGui 字体与绘图执行生产 draw_health；校验中文缺字和单行几何，并生成离线预览数据。
const fs=require('fs'),path=require('path'),cp=require('child_process'),assert=require('assert/strict');
const project=path.resolve(__dirname,'..'),out=path.resolve(process.argv[2]||path.join(project,'target/info-layout'));
const fixture=path.resolve(process.argv[3]||path.join(project,'conf/target_health_display.ini'));
assert.ok(process.argv[4],'用法：node tools/verify-info-layout.cjs "输出目录" "INI模板" "Forge前置源码目录"；需要 C++ 编译器，可用 GXX 指定');
const forgeSource=path.resolve(process.argv[4]);
fs.mkdirSync(out,{recursive:true});
const gui=path.join(forgeSource,'libs/cimgui/imgui');
const rustPath=p=>JSON.stringify(p.replaceAll(String.fromCharCode(92),'/'));
const cpp=String.raw`
#include "imgui.h"
#include "imgui_internal.h"
#include <cassert>
#include <cstdio>
#include <string>
static ImFont* fonts[2];static int missing=0,text_calls=0;static float boxes[2][4];static FILE* log_file;
#define API extern "C" __declspec(dllexport)
API void ui_init(const char* latin,const char* chinese){
 IMGUI_CHECKVERSION();ImGui::CreateContext();auto& io=ImGui::GetIO();io.IniFilename=nullptr;io.DeltaTime=1.f/60;
 io.BackendFlags|=ImGuiBackendFlags_RendererHasTextures;
 fonts[0]=io.Fonts->AddFontFromFileTTF(latin,17);fonts[1]=io.Fonts->AddFontFromFileTTF(chinese,17);assert(fonts[0]&&fonts[1]);
 assert(!fonts[0]->IsGlyphInFont(0x9F99));assert(fonts[1]->IsGlyphInFont(0x9F99));
}
API void ui_begin(float w,float h,const char* log){
 ImGui::GetIO().DisplaySize=ImVec2(w,h);ImGui::NewFrame();ImGui::Begin("offline",nullptr,ImGuiWindowFlags_NoBackground|ImGuiWindowFlags_NoSavedSettings);
 missing=text_calls=0;log_file=fopen(log,"wb");assert(log_file);fprintf(log_file,"{\"width\":%.1f,\"height\":%.1f,\"draw\":[",w,h);
}
API void ui_push(int font,float size){ImGui::PushFont(fonts[font],size);}API void ui_pop(){ImGui::PopFont();}
API void ui_measure(const char* text,int len,float* result){auto p=ImGui::CalcTextSize(text,text+len);result[0]=p.x;result[1]=p.y;}
API void ui_clip(float x,float y,float r,float b,bool intersect){ImGui::GetForegroundDrawList()->PushClipRect(ImVec2(x,y),ImVec2(r,b),intersect);}API void ui_unclip(){ImGui::GetForegroundDrawList()->PopClipRect();}
static void comma(){if(text_calls++)fputc(',',log_file);}
API void ui_rect(float x,float y,float r,float b,unsigned color,float rounding,float thickness,bool filled){
 auto d=ImGui::GetForegroundDrawList();if(filled)d->AddRectFilled(ImVec2(x,y),ImVec2(r,b),color,rounding);else d->AddRect(ImVec2(x,y),ImVec2(r,b),color,rounding,0,thickness);
 comma();fprintf(log_file,"{\"kind\":\"rect\",\"x\":%.3f,\"y\":%.3f,\"right\":%.3f,\"bottom\":%.3f,\"color\":%u,\"rounding\":%.3f,\"thickness\":%.3f,\"filled\":%s}",x,y,r,b,color,rounding,thickness,filled?"true":"false");
}
API void ui_text(int font,float size,float x,float y,unsigned color,const char* text,int len){
 for(const char* p=text;p<text+len;){unsigned c;int n=ImTextCharFromUtf8(&c,p,text+len);assert(n>0);p+=n;
   if(c>32 && !fonts[font]->GetFontBaked(size)->FindGlyphNoFallback(c))missing++;}
 auto bounds=fonts[font]->CalcTextSizeA(size,FLT_MAX,0,text,text+len);boxes[font][0]=x;boxes[font][1]=y;boxes[font][2]=bounds.x;boxes[font][3]=bounds.y;
 ImGui::GetForegroundDrawList()->AddText(fonts[font],size,ImVec2(x,y),color,text,text+len);
 comma();fprintf(log_file,"{\"kind\":\"text\",\"font\":%d,\"size\":%.3f,\"x\":%.3f,\"y\":%.3f,\"width\":%.3f,\"height\":%.3f,\"color\":%u,\"text\":\"",font,size,x,y,bounds.x,bounds.y,color);
 for(int i=0;i<len;i++){if(text[i]=='\"'||text[i]=='\\')fputc('\\',log_file);if(text[i]=='\n'){fputs("\\n",log_file);}else fputc(text[i],log_file);}fputs("\"}",log_file);
}
API void ui_box(int font,float* out){for(int i=0;i<4;i++)out[i]=boxes[font][i];}
API int ui_end(){ImGui::End();ImGui::Render();fprintf(log_file,"],\"missing_glyphs\":%d}",missing);fclose(log_file);return missing;}
`;
const shim=path.join(out,'info-font.cpp'),dll=path.join(out,'info-font.dll');
const changed=!fs.existsSync(shim)||fs.readFileSync(shim,'utf8')!==cpp;fs.writeFileSync(shim,cpp);
const cpps=['imgui.cpp','imgui_draw.cpp','imgui_tables.cpp','imgui_widgets.cpp'].map(n=>path.join(gui,n));
if(changed||!fs.existsSync(dll)||cpps.some(f=>fs.statSync(f).mtimeMs>fs.statSync(dll).mtimeMs))cp.execFileSync(process.env.GXX||'g++',['-std=c++17','-O0','-shared','-static-libgcc','-static-libstdc++','-DIMGUI_DISABLE_WIN32_FUNCTIONS','-DIMGUI_DISABLE_DEFAULT_SHELL_FUNCTIONS','-I',gui,shim,...cpps,'-o',dll],{stdio:'inherit'});
const lib=fs.readFileSync(path.join(project,'src/lib.rs'),'utf8');
const draw=lib.slice(lib.indexOf('fn draw_health()'),lib.indexOf('#[forge::imgui_render]'));
assert.ok(draw.includes('fn draw_monster_info(')&&draw.includes('info_origin('));
const rust=String.raw`
#![allow(dead_code)]
use std::{ffi::{c_char,c_void},sync::{OnceLock,atomic::Ordering}};
#[link(name="kernel32")]unsafe extern "system"{fn LoadLibraryA(p:*const c_char)->*mut c_void;fn GetProcAddress(m:*mut c_void,n:*const c_char)->*mut c_void;}
struct Api{init:unsafe extern "C" fn(*const c_char,*const c_char),begin:unsafe extern "C" fn(f32,f32,*const c_char),push:unsafe extern "C" fn(i32,f32),pop:unsafe extern "C" fn(),measure:unsafe extern "C" fn(*const c_char,i32,*mut f32),clip:unsafe extern "C" fn(f32,f32,f32,f32,bool),unclip:unsafe extern "C" fn(),rect:unsafe extern "C" fn(f32,f32,f32,f32,u32,f32,f32,bool),text:unsafe extern "C" fn(i32,f32,f32,f32,u32,*const c_char,i32),end:unsafe extern "C" fn()->i32,box_:unsafe extern "C" fn(i32,*mut f32)}
static API:OnceLock<Api>=OnceLock::new();fn api()->&'static Api{API.get().unwrap()}
#[derive(Clone,Copy)]struct ImVec2{x:f32,y:f32}impl ImVec2{fn new(x:f32,y:f32)->Self{Self{x,y}}}
enum ImDrawFlags{None}
mod imgui{
 use super::*;#[derive(Clone,Copy)]pub struct ImFont(pub i32);impl ImFont{pub fn current()->Self{Self(0)}}
 pub fn push_font(f:&ImFont,s:Option<f32>){unsafe{(api().push)(f.0,s.unwrap())}}pub fn pop_font(){unsafe{(api().pop)()}}
 pub fn calc_text_size(s:&str,_:bool,_:f32)->ImVec2{let mut p=[0.;2];unsafe{(api().measure)(s.as_ptr().cast(),s.len() as i32,p.as_mut_ptr())};ImVec2::new(p[0],p[1])}
 pub fn color(r:u8,g:u8,b:u8,a:u8)->u32{r as u32|((g as u32)<<8)|((b as u32)<<16)|((a as u32)<<24)}
 pub struct DrawList;impl DrawList{pub fn foreground()->Self{Self}
 pub fn push_clip_rect(&self,a:ImVec2,b:ImVec2,i:bool){unsafe{(api().clip)(a.x,a.y,b.x,b.y,i)}}pub fn pop_clip_rect(&self){unsafe{(api().unclip)()}}
 pub fn add_rect_filled(&self,a:ImVec2,b:ImVec2,c:u32,r:f32){unsafe{(api().rect)(a.x,a.y,b.x,b.y,c,r,0.,true)}}
 pub fn add_rect(&self,a:ImVec2,b:ImVec2,c:u32,r:f32,t:f32,_:ImDrawFlags){unsafe{(api().rect)(a.x,a.y,b.x,b.y,c,r,t,false)}}
 pub fn add_text_with_font(&self,f:&ImFont,size:f32,p:ImVec2,c:u32,s:&str){unsafe{(api().text)(f.0,size,p.x,p.y,c,s.as_ptr().cast(),s.len() as i32)}}}
}
static mut CUSTOM_FONT:Option<imgui::ImFont>=None;static mut INFO_FONT:Option<imgui::ImFont>=Some(imgui::ImFont(1));
unsafe fn loading_visible(_:u32)->bool{false}
`+`#[path=${rustPath(path.join(project,'src/data.rs'))}]mod data;use data::*;\n`+draw+`
fn main(){unsafe{
 let module=LoadLibraryA(std::ffi::CString::new(${rustPath(dll)}).unwrap().as_ptr());assert!(!module.is_null());
 macro_rules! sym{($s:literal)=>{std::mem::transmute(GetProcAddress(module,concat!($s,"\\0").as_ptr().cast()))};}
 let a=Api{init:sym!("ui_init"),begin:sym!("ui_begin"),push:sym!("ui_push"),pop:sym!("ui_pop"),measure:sym!("ui_measure"),clip:sym!("ui_clip"),unclip:sym!("ui_unclip"),rect:sym!("ui_rect"),text:sym!("ui_text"),end:sym!("ui_end"),box_:sym!("ui_box")};assert!(API.set(a).is_ok());
 (api().init)(std::ffi::CString::new(${rustPath(path.join(forgeSource,'data/fonts/Roboto-Medium.ttf'))}).unwrap().as_ptr(),std::ffi::CString::new(${rustPath(path.join(project,'assets/romfs/nativeNX/forge/fonts/NotoSansSC-Medium.otf'))}).unwrap().as_ptr());
 let config=Config::parse(include_str!(${rustPath(fixture)})).unwrap();
 assert_eq!(config,Config::DEFAULT,"all defaults must equal the supplied INI");
 let info=MonsterInfo{name:Text::from_str("雷狼龙"),species:Text::from_str("牙龙种"),scale:1.12,size_cm:1991.47,crown:0xFF,enraged:false};
 let mut logs=Vec::new();
 for (id,w,h) in [("handheld",1280.,720.),("docked",1920.,1080.),("large-text",1280.,720.),("compact",1280.,720.),("hidden-hp",1280.,720.),("manual-y",1280.,720.)]{
  let mut config=config;let mut info=info;
  match id{
   "large-text"=>{config.info.font_size=100.;config.info.show_species=true;config.info.show_scale=true;config.info.show_size=true;},
   "compact"=>{info.name=Text::from_str("天迴龙");config.info.show_species=false;config.info.show_scale=false;config.info.show_size=false;},
   "hidden-hp"=>{config.show_values=false;config.show_percent=false;},
   "manual-y"=>config.info.offset_y=-12.,
   _=>{}
  }
  let log=std::ffi::CString::new(format!("{}/{}.json",${rustPath(out)},id)).unwrap();(api().begin)(w,h,log.as_ptr());
  STATE.with(|s|{s.config=config;s.snapshot=Some(Snapshot{health:Health::new(5131,7975).unwrap(),info,display:DisplayArea{width:w,height:h}})});ENABLED.store(true,Ordering::Relaxed);
  draw_health();let mut hp=[0.;4];let mut row=[0.;4];(api().box_)(0,hp.as_mut_ptr());(api().box_)(1,row.as_mut_ptr());
  assert!(row[0]>=0. && row[0]+row[2]<=w+1.0,"single row must fit viewport");
  let expected_y=(config.y+config.height+config.info.line_spacing+config.info.offset_y)*(h/720.);
  assert!((row[1]-expected_y).abs()<0.01,"info Y must honor its own configuration without reserving an HP row");
  if id=="compact"{assert!(row[0]+row[2]<hp[0],"left-side information and right-side HP fit side by side");assert!(row[1]<hp[1]+hp[3] && row[1]+row[3]>hp[1],"compact information must stay beside HP instead of moving below it");}
  assert_eq!((api().end)(),0,"Chinese glyph missing in actual font rasterization");logs.push(id);
 }
 println!("PASS: actual draw_health with Dear ImGui; all defaults equal supplied INI, Chinese glyphs rasterize, one horizontal info row, configured Y honored with and without HP: {:?}",logs);
}}
`;
const file=path.join(out,'info-layout.rs'),exe=path.join(out,'info-layout.exe');fs.writeFileSync(file,rust);
cp.execFileSync(process.env.RUSTC||'rustc',['--edition=2024',file,'-o',exe],{stdio:'inherit'});
cp.execFileSync(exe,{stdio:'inherit'});
for(const name of ['handheld','docked','large-text','compact','hidden-hp','manual-y']){
 const data=JSON.parse(fs.readFileSync(path.join(out,name+'.json')));const info=data.draw.filter(x=>x.kind==='text'&&x.font===1);
 assert.ok(info.length>0);assert.ok(info.every(x=>!x.text.includes('\n')));assert.equal(data.missing_glyphs,0);
}
fs.writeFileSync(path.join(out,'verification.json'),JSON.stringify({actual_production_draw:true,actual_imgui:true,roboto_missing_chinese_reproduced:true,cjk_missing_glyphs:0,single_horizontal_info_row:true,user_defaults_identical:true,fixture,info_y_follows_configuration:true,compact_info_beside_hp:true,hp_visibility_does_not_move_info:true,manual_y_offset_honored:true,viewports:['handheld','docked','large-text'],game_tested:false},null,2)+'\n');
