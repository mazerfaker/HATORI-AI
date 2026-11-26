use eframe::egui::{self, Color32, ViewportCommand as Cmd};
use super::Config;

pub struct Presence {
    pub error: Option<String>,
    hidden: bool,
    exiting: bool,
    #[cfg(windows)]
    tray: Option<tray_icon::TrayIcon>,
    #[cfg(windows)]
    ids: Vec<tray_icon::menu::MenuId>,
}
impl Presence {
    pub fn new() -> Self {
        let mut p = Self { error: None, hidden: false, exiting: false,
            #[cfg(windows)] tray: None, #[cfg(windows)] ids: Vec::new() };
        #[cfg(windows)] {
            use tray_icon::{Icon, TrayIconBuilder, menu::{Menu, MenuItem}};
            let menu = Menu::new();
            let items = [MenuItem::new("Open Hatoriel", true, None), MenuItem::new("Toggle avatar", true, None), MenuItem::new("Exit Hatoriel", true, None)];
            p.ids = items.iter().map(|i| i.id().clone()).collect();
            let result = (|| -> Result<tray_icon::TrayIcon, Box<dyn std::error::Error>> {
                for item in &items { menu.append(item)?; }
                let rgba = super::crimson::icon_rgba();
                Ok(TrayIconBuilder::new().with_menu(Box::new(menu)).with_tooltip("Hatoriel").with_icon(Icon::from_rgba(rgba,32,32)?).build()?)
            })();
            match result { Ok(t)=>p.tray=Some(t), Err(e)=>p.error=Some(format!("Tray unavailable: {e}")) }
        }
        p
    }
    pub fn available(&self)->bool { #[cfg(windows)] {self.tray.is_some()} #[cfg(not(windows))] {false} }
    pub fn hide(&mut self, ctx:&egui::Context) { if self.available() { self.hidden=true; ctx.send_viewport_cmd(Cmd::Minimized(true)); } }
    fn restore(&mut self,ctx:&egui::Context) { self.hidden=false; ctx.send_viewport_cmd(Cmd::Visible(true)); ctx.send_viewport_cmd(Cmd::Minimized(false)); ctx.send_viewport_cmd(Cmd::Focus); }
    pub fn update(&mut self,ctx:&egui::Context,cfg:&mut Config,busy:bool)->bool {
        let mut changed=false;
        #[cfg(windows)] {
            use tray_icon::{TrayIconEvent, menu::MenuEvent};
            while let Ok(e)=MenuEvent::receiver().try_recv() {
                match self.ids.iter().position(|id| id==&e.id) { Some(0)=>self.restore(ctx), Some(1)=>{cfg.avatar=!cfg.avatar;changed=true;}, Some(2)=>{self.exiting=true;ctx.send_viewport_cmd(Cmd::Close);}, _=>{} }
            }
            while let Ok(e)=TrayIconEvent::receiver().try_recv() { if matches!(e,TrayIconEvent::DoubleClick{..}) {self.restore(ctx);} }
        }
        if !self.exiting && self.available() && cfg.minimize_to_tray {
            if ctx.input(|i|i.viewport().close_requested()) {ctx.send_viewport_cmd(Cmd::CancelClose);self.hide(ctx);}
            if !self.hidden && ctx.input(|i|i.viewport().minimized==Some(true)) {self.hide(ctx);}
            if self.hidden && ctx.input(|i|i.viewport().minimized==Some(false)) {self.hidden=false;}
        }
        if cfg.avatar && !self.exiting {
            ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("hatori-avatar"),egui::ViewportBuilder::default().with_title("Hatoriel Avatar").with_inner_size([180.,205.]).with_decorations(false).with_transparent(true).with_resizable(false).with_always_on_top(),|ctx,_class| {
                if ctx.input(|i|i.viewport().close_requested()) {cfg.avatar=false;changed=true;}
                egui::CentralPanel::default().frame(egui::Frame::none().fill(Color32::TRANSPARENT)).show(ctx,|ui| {
                    let (r,response)=ui.allocate_exact_size(egui::vec2(180.,160.),egui::Sense::click_and_drag());
                    if response.drag_started(){ctx.send_viewport_cmd(Cmd::StartDrag);}
                    let p=ui.painter();let c=r.center();let t=ctx.input(|i|i.time) as f32;
                    super::crimson::reticle(p,c,34.,t,busy);
                    p.text(c+egui::vec2(0.,65.),egui::Align2::CENTER_CENTER,if busy{"THINKING"}else{"Hatoriel"},egui::FontId::monospace(12.),super::crimson::RED);
                    ui.horizontal(|ui|{if ui.small_button("Open").clicked(){ctx.send_viewport_cmd_to(egui::ViewportId::ROOT,Cmd::Visible(true));ctx.send_viewport_cmd_to(egui::ViewportId::ROOT,Cmd::Minimized(false));ctx.send_viewport_cmd_to(egui::ViewportId::ROOT,Cmd::Focus);self.hidden=false;} if ui.small_button("Hide").clicked(){cfg.avatar=false;changed=true;}});
                });ctx.request_repaint_after(std::time::Duration::from_millis(50));
            });
        }
        changed
    }
}
