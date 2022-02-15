extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;
use nwg::CheckBoxState::{Checked, Unchecked};
use nwd::NwgUi;
use nwg::stretch::{geometry::{Size, Rect}, style::{Dimension as D, FlexDirection}};

const PT_10: D = D::Points(10.0);
const PT_5: D = D::Points(5.0);
const PT_0: D = D::Points(0.0);
const PT_28: D = D::Points(28.0);
const MARGIN: Rect<D> = Rect{ start: PT_5, end: PT_5, top: PT_5, bottom: PT_0 };
const WINDOW_LAYOUT_PADDING: Rect<D> = Rect{ start: PT_0, end: PT_10, top: PT_0, bottom: PT_28 };

const STARTUP_WINDOW_X: i32 = -100;
const STARTUP_WINDOW_Y: i32 = -100;
const DEFAULT_WINDOW_WIDTH: u32 = 300;
const DEFAULT_WINDOW_HEIGHT: u32 = 300;
const WINDOW_TITLE: &str = "SVT";
const SVT_OPTIONS_FILE: &str = "svt_config.txt";

use anyhow::{anyhow, Result};
use serde::{Serialize, Deserialize};
use libxch;

use std::{cell::RefCell};
use std::cmp;
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use crate::svt;

//TODO consider using this as a general config parameter to pass around in functions involving SVT
#[derive(Serialize, Deserialize, Debug)]
pub struct AppOptions {
  pub map: String,
  pub inh_times: String,
  pub lin_sv: bool,
  pub exp_sv: bool,
  pub pol_sv: bool,
  pub flat_sv: bool,
  pub vol: bool,
  pub hits: bool,
  pub barlines: bool,
  pub inh_lines: bool,
  pub offset: String,
  pub buffer: String,
  pub min_spacing: String,
  pub pol_exp: String,
  pub flat_change: String,
  pub ignore_bpm: bool,
  pub pos_x: i32,
  pub pos_y: i32,
  pub width: u32,
  pub height: u32,
  pub experimental: String,
}

impl Default for AppOptions {
  fn default() -> Self { AppOptions {
      map: String::from(""),
      inh_times: String::from(""),
      lin_sv: true,
      pol_sv: false,
      exp_sv: false,
      flat_sv: false,
      vol: false,
      hits: true,
      barlines: false,
      inh_lines: false,
      offset: String::from("-1"),
      buffer: String::from("3"),
      min_spacing: String::from("3"),
      pol_exp: String::from("0.5"),
      flat_change: String::from("0.0"),
      ignore_bpm: false,
      pos_x: cmp::max(0, nwg::Monitor::width() / 2 - (DEFAULT_WINDOW_WIDTH / 2) as i32),
      pos_y: cmp::max(0, nwg::Monitor::height() / 2 - (DEFAULT_WINDOW_HEIGHT / 2) as i32),
      width: DEFAULT_WINDOW_WIDTH,
      height: DEFAULT_WINDOW_HEIGHT,
      experimental: String::from(""),
    }
  }
}

#[derive(Default, NwgUi)]
pub struct UI {
  //start window as 0-sized and off-screen, then move on-screen after config is loaded to prevent flashing
  #[nwg_control(size: (0,0), position: (STARTUP_WINDOW_X, STARTUP_WINDOW_Y), title: WINDOW_TITLE, accept_files: true, flags: "WINDOW|VISIBLE|MINIMIZE_BOX|RESIZABLE")]
  #[nwg_events( OnWindowClose: [UI::close_window], OnFileDrop: [UI::drop_file(SELF, EVT_DATA)], OnResizeBegin: [UI::resize_begin], OnResizeEnd: [UI::resize_end] )]
  pub window: nwg::Window,

  #[nwg_layout(parent: window, flex_direction: FlexDirection::Column, padding: WINDOW_LAYOUT_PADDING)]
  window_layout: nwg::FlexboxLayout,

  //timing point input
  #[nwg_control(text: "", flags: "VISIBLE|AUTOVSCROLL|TAB_STOP")]
  #[nwg_layout_item(layout: window_layout, margin: MARGIN,
    size: Size { width: D::Percent(1.0), height: D::Percent(0.15) },
    flex_grow: 1.0,
  )]
  pub inherited_text: nwg::TextBox,

  #[nwg_control(flags: "VISIBLE")]
  #[nwg_layout_item(layout: window_layout, margin: MARGIN,
    size: Size { width: D::Percent(1.0), height: D::Points(120.0) },
  )]
  pub options_frame: nwg::Frame,

  #[nwg_control(flags: "VISIBLE")]
  #[nwg_layout_item(layout: window_layout, margin: MARGIN,
    size: Size { width: D::Percent(1.0), height: D::Points(55.0) },
  )]
  pub mapselect_frame: nwg::Frame,

  #[nwg_control(flags: "VISIBLE")]
  #[nwg_layout_item(layout: window_layout, margin: MARGIN,
    size: Size { width: D::Percent(1.0), height: D::Points(25.0) },
  )]
  pub applyundo_frame: nwg::Frame,

  //outline around the apply controls
  #[nwg_control(size: (60, 120), position: (0, 0), parent: options_frame)]
  pub apply_frame: nwg::Frame,

  #[nwg_control(text: "Apply:", size: (45, 20), position: (2, 0), parent: apply_frame)]
  pub apply_label: nwg::Label,

  //toggles sv changes
  #[nwg_control(text: "Lin. SV", size: (95, 20), position: (2, 20), check_state: Checked, parent: apply_frame)]
  #[nwg_events(OnButtonClick: [UI::set_sv_mode(SELF, CTRL), UI::update_config(SELF)])]
  pub lin_sv_check: nwg::CheckBox,

  //toggles sv changes
  #[nwg_control(text: "Exp. SV", size: (95, 20), position: (2, 40), check_state: Unchecked, parent: apply_frame)]
  #[nwg_events(OnButtonClick: [UI::set_sv_mode(SELF, CTRL), UI::update_config(SELF)])]
  pub exp_sv_check: nwg::CheckBox,

  //toggles sv changes
  #[nwg_control(text: "Pol. SV", size: (95, 20), position: (2, 60), check_state: Unchecked, parent: apply_frame)]
  #[nwg_events(OnButtonClick: [UI::set_sv_mode(SELF, CTRL), UI::update_config(SELF)])]
  pub pol_sv_check: nwg::CheckBox,

      //toggles sv changes
  #[nwg_control(text: "Flat SV", size: (95, 20), position: (2, 80), check_state: Unchecked, parent: apply_frame)]
  #[nwg_events(OnButtonClick: [UI::set_sv_mode(SELF, CTRL), UI::update_config(SELF)])]
  pub flat_sv_check: nwg::CheckBox,

  //toggles vol changes
  #[nwg_control(text: "Lin. Vol", size: (95, 20), position: (2, 100), check_state: Checked, parent: apply_frame)]
  #[nwg_events(OnButtonClick: [UI::update_config(SELF)])]
  pub vol_check: nwg::CheckBox,

  //outline around the apply to controls
  #[nwg_control(size: (70, 120), position: (59, 0), parent: options_frame)]
  pub apply_to_frame: nwg::Frame,

  #[nwg_control(text: "To:", size: (65, 20), position: (2, 0), parent: apply_to_frame)]
  pub apply_to_label: nwg::Label,

  //toggles note changes
  #[nwg_control(text: "Hits", size: (95, 20), position: (2, 20), check_state: Checked, parent: apply_to_frame)]
  #[nwg_events(OnButtonClick: [UI::update_config(SELF)])]
  pub hit_check: nwg::CheckBox,

  //toggles barline changes
  #[nwg_control(text: "Barlines", size: (95, 20), position: (2, 40), check_state: Checked, parent: apply_to_frame)]
  #[nwg_events(OnButtonClick: [UI::update_config(SELF)])]
  pub barline_check: nwg::CheckBox,

  //toggles inh line changes
  #[nwg_control(text: "Inh. lines", size: (95, 20), position: (2, 60), check_state: Unchecked, parent: apply_to_frame)]
  #[nwg_events(OnButtonClick: [UI::update_config(SELF)])]
  pub inh_check: nwg::CheckBox,

  //outline around advanced controls
  #[nwg_control(size: (162, 120), position: (128, 0), parent: options_frame)]
  pub advanced_options_frame: nwg::Frame,

  #[nwg_control(text: "Advanced Options:", size: (195, 20), position: (2, 0), parent: advanced_options_frame)]
  pub advanced_options_label: nwg::Label,

  //offset time
  #[nwg_control(text: "", size: (19, 19), position: (2, 20), parent: advanced_options_frame)]
  #[nwg_events(OnTextInput: [UI::update_config(SELF)])]
  pub offset_text: nwg::TextInput,

  #[nwg_control(text: "Offset", size: (45, 20), position: (25, 22), parent: advanced_options_frame)]
  pub offset_label: nwg::Label,

  //buffer time
  #[nwg_control(text: "", size: (19, 19), position: (2, 40), parent: advanced_options_frame)]
  #[nwg_events(OnTextInput: [UI::update_config(SELF)])]
  pub buffer_text: nwg::TextInput,

  #[nwg_control(text: "Buffer", size: (45, 20), position: (25, 42), parent: advanced_options_frame)]
  pub buffer_label: nwg::Label,

  #[nwg_control(text: "", size: (19, 19), position: (2, 60), parent: advanced_options_frame)]
  #[nwg_events(OnTextInput: [UI::update_config(SELF)])]
  pub min_spacing_text: nwg::TextInput,

  #[nwg_control(text: "Min. Spacing", size: (85, 20), position: (25, 62), parent: advanced_options_frame)]
  pub min_spacing_label: nwg::Label,

  //exponential factor
  #[nwg_control(text: "", size: (19, 19), position: (2, 80), parent: advanced_options_frame)]
  #[nwg_events(OnTextInput: [UI::update_config(SELF)])]
  pub pol_exp_text: nwg::TextInput,

  #[nwg_control(text: "Exp.", size: (45, 20), position: (25, 82), parent: advanced_options_frame)]
  pub pol_exp_label: nwg::Label,

  #[nwg_control(text: "", size: (19, 19), position: (2, 80), parent: advanced_options_frame)]
  #[nwg_events(OnTextInput: [UI::update_config(SELF)])]
  pub flat_sv_text: nwg::TextInput,

  #[nwg_control(text: "SV Change", size: (100, 20), position: (25, 82), parent: advanced_options_frame)]
  pub flat_sv_label: nwg::Label,

  //toggles end line/start line BPM
  #[nwg_control(text: "Ignore BPM", size: (105, 20), position: (75, 20), check_state: Unchecked, parent: advanced_options_frame)]
  #[nwg_events(OnButtonClick: [UI::update_config(SELF)])]
  pub ign_bpm_check: nwg::CheckBox,

  //select map button
  #[nwg_control(text: "Select Map", size: (87, 25), position: (-1,0), parent: mapselect_frame)]
  #[nwg_events( OnButtonClick: [UI::open_file_browser] )]
  pub open_button: nwg::Button,

  //input map filename
  #[nwg_control(text: "", size: (200, 23), position: (90, 1), flags: "VISIBLE|DISABLED", parent: mapselect_frame)]
  pub in_filename: nwg::TextInput,

  //toggles preview
  #[nwg_control(text: "Preview Diff", size: (87, 25), position: (0, 30), check_state: Unchecked, parent: mapselect_frame)]
  #[nwg_events(OnButtonClick: [UI::fill_out_filename, UI::update_config(SELF)])]
  pub preview_check: nwg::CheckBox,
  
  //output map filename
  #[nwg_control(text: "", size: (200, 23), position: (90, 31), parent: mapselect_frame)]
  #[nwg_events(OnTextInput: [UI::update_config(SELF)])]
  pub out_filename: nwg::TextInput,
  
  //place apply button near bottom
  #[nwg_control(text: "Apply", size: (242, 25), position: (0, 0), flags: "VISIBLE|DISABLED", parent: applyundo_frame)]
  #[nwg_events( OnButtonClick: [UI::apply_changes] )]
  pub apply_button: nwg::Button,

  //place undo/redo button near bottom
  #[nwg_control(text: "Undo", size: (45, 25), position: (247, 0), flags: "VISIBLE|DISABLED", parent: applyundo_frame)]
  #[nwg_events( OnButtonClick: [UI::undo] )]
  pub undo_button: nwg::Button,

  //place status bar at the very bottom
  #[nwg_control(text: "[map] no map selected (Select Map or drag one in)")]
  pub status: nwg::StatusBar,

  //open file dialog
  #[nwg_resource(title: "Open File", action: nwg::FileDialogAction::Open, filters: "osu(*.osu)")]
  pub file_dialog: nwg::FileDialog,

  pub options: RefCell<AppOptions>, //reference to options which are updated when UI elements are clicked
  pub svt: RefCell<svt::SVT>, //reference to svt which contains logic for tool
  pub pos_x: RefCell<i32>,
  pub pos_y: RefCell<i32>,
}

impl UI {
  pub fn init(&self, svt_app: svt::SVT) {
    //set icon on taskbar and on window top left
    let icon_bytes = include_bytes!("../assets/svt.ico");
    let mut icon = nwg::Icon::default();
    let _res_ = nwg::Icon::builder()
      .source_bin(Some(icon_bytes))
      .strict(true)
      .build(&mut icon);
    self.window.set_icon(Some(&icon));

    //load config and set apply button accordingly
    if self.load_config().is_err() {
      println!("[load] couldn't load config properly");
      self.apply_button.set_enabled(false);
    }

    let options = self.options.borrow_mut();
    self.window.set_size(options.width, options.height);
    self.window.set_position(options.pos_x, options.pos_y);

    //always disable undo button by default
    self.undo_button.set_enabled(false);

    self.set_sv_mode(&self.lin_sv_check);

    self.svt.replace(svt_app);
  }

  fn apply_changes(&self) {
    //refresh file before doing anything
    self.load_file();

    if self.svt.borrow().all_objs.len() == 0 {
      self.status.set_text(0, &format!("[apply] no objects loaded, please check map is valid"));
      return;
    }

    //[debug] print out all objects in their current order
    self.svt.borrow().print_debug();

    let cmd = self.inherited_text.text();
    let mut lines = cmd.split_whitespace();
    let mut start_line;
    let mut end_line;

    //process 2 valid lines at a time until no lines left
    loop {
      start_line = lines.next();
      end_line = lines.next();
      if let (Some(start_l), Some(end_l)) = (start_line, end_line) {
        if let Err(err) = self.svt.borrow_mut().apply_timing(start_l, end_l, &*self.options.borrow()) {
          //if error is encountered, stop applying and update status bar
          println!("[apply] error applying timing {} -> {}", start_l, end_l);
          self.status.set_text(0, &err.to_string());
          return;
        }
      } else {
        println!("[apply] no more lines");
        break;
      }
    }

    //merge new points into old ones - delete old point if new one is identical
    let write_result = self.svt.borrow_mut().write_output_points(self.min_spacing_text.text(), self.in_filename.text(), self.out_filename.text(), self.preview_check.check_state() == Checked);
    
    if write_result.is_err() {
      println!("[apply] error writing output");
      self.status.set_text(0, &write_result.unwrap_err().to_string());
      return;
    }

    //save config after successful output point write
    if self.save_config().is_err() {
      self.status.set_text(0, &format!("[apply] couldn't save config"));
      return;
    }

    //enable undo button
    self.undo_button.set_enabled(true);

    //update status bar with change count on success
    self.status.set_text(0, &format!("[apply] {} lines applied", write_result.unwrap()));
  }
  
  fn close_window(&self) {
    self.update_config();

    if self.save_config().is_err() {
      println!("[close] failed to save config");
      return;
    }
    nwg::stop_thread_dispatch();
  }

  fn drop_file(&self, data: &nwg::EventData) {
    self.in_filename.set_text(&data.on_file_drop().files().pop().unwrap_or(String::from("failed_to_import_for_some_reason.osu")));
    self.fill_out_filename();
    self.load_file();
  }

  fn fill_out_filename(&self) {
    if self.preview_check.check_state() == Checked {
      let in_filename = &self.in_filename.text();

      //prevent paths without parents or filenames from crashing
      let path_folder = Path::new(in_filename).parent();
      let path_osu = Path::new(in_filename).file_name();

      //TODO check path is valid maybe?
      //TODO fix bracket validation - file without brackets should be cut at .osu instead
      if let (Some(path_folder), Some(path_osu)) = (path_folder, path_osu) {
        
        if let Some(name_osu) = path_osu.to_str() {
          let preview_cut = if name_osu.contains("[") {
            name_osu.split("[").nth(0).unwrap_or("")
          } else {
            name_osu.split(".").nth(0).unwrap_or("")
          };

          self.out_filename.set_text(&format!("{}/{}[{}].osu", path_folder.to_str().unwrap_or(""), preview_cut, "preview"));
        } else {
          println!("[fof] path name didn't unwrap correctly");
        }
      } else {
        println!("[fof] issue with either file directory or name: [{}]", in_filename);
        self.status.set_text(0, &format!("[fof] issue with input filename"));
      }
    } else {
      self.out_filename.set_text(&self.in_filename.text());
    }
  }

  //validate, load, and parse .osu file line by line
  fn load_file(&self) {
    let filename = self.in_filename.text();

    //should never happen
    if filename.len() == 0 {
      println!("[load] empty filename");
      return;
    }

    //determine filename and extension
    let ext = Path::new(&filename).extension();

    //skip any file that is not .osu
    if ext != Some(OsStr::new("osu")) {
      println!("[load] invalid file");
      self.apply_button.set_enabled(false);
      self.status.set_text(0, &format!("[load] please select a .osu file"));
      return;
    }

    //let folder = String::from(Path::new(&filename).parent().unwrap().to_str().unwrap());
    
    let path_folder = Path::new(&filename).parent();
    let path_osu = Path::new(&filename).file_name();

    if let (Some(path_folder), Some(path_osu)) = (path_folder, path_osu) {
      println!("[load] folder: {}", path_folder.to_str().unwrap_or("folder_dne"));
      println!("[load] file: {}", path_osu.to_str().unwrap_or("filename_dne.osu"));
      println!("[load] load starting");
      self.svt.borrow_mut().load_osu(&filename);

      if self.save_config().is_err() {
        self.status.set_text(0, &format!("[apply] couldn't save config"));
        return;
      }
      self.apply_button.set_enabled(true);
      self.status.set_text(0, &format!("editing {}", path_osu.to_str().unwrap_or("filename_dne.osu")));
    } else {
      self.status.set_text(0, &format!("[load] issue with either file directory or name"));
    }
  }

  fn open_file_browser(&self) {
    if let Ok(d) = env::current_dir() {
      if let Some(d) = d.to_str() {
        self.file_dialog.set_default_folder(d).expect("[brow] failed to set default folder");
      }
    }
  
    if self.file_dialog.run(Some(&self.window)) {
      self.in_filename.set_text("");
      if let Ok(selected) = self.file_dialog.get_selected_item() {
        if let Ok(selected_str) = selected.into_string() {
          self.in_filename.set_text(&selected_str);
          self.fill_out_filename();
          self.load_file();
        } else {
          self.status.set_text(0, &format!("[load] failed to load file: into_string failed"));
        }        
      } else {
        self.status.set_text(0, &format!("[load] failed to load file: get_selected_item failed"));
      }
    }
  }

  //assumes a map is loaded and a change has been applied already
  fn undo(&self) {
    if let Err(e) = libxch::xch(self.in_filename.text(), "backup.osu") {
      self.status.set_text(0, &format!("[undo] failed {}", e.to_string()));
    }
    //fs::copy(self.in_filename.text(), "temp.osu")?;
    //fs::copy("backup.osu", self.in_filename.text())?;
    //fs::copy("temp.osu", "backup.osu")?;
    //fs::remove_file("temp.osu")?;

    self.undo_button.set_enabled(false);
    self.status.set_text(0, &format!("[undo] successful"));
  }

  fn load_config(&self) -> Result<()> {
    // read file
    let app_options_string = fs::read_to_string(SVT_OPTIONS_FILE).unwrap_or(String::from(""));
    let mut app_options = serde_json::from_str(&app_options_string).unwrap_or(AppOptions{..Default::default()});

    self.inherited_text.set_text(&app_options.inh_times);
    self.in_filename.set_text(&app_options.map);
    self.lin_sv_check.set_check_state(if app_options.lin_sv {Checked} else {Unchecked});
    self.pol_sv_check.set_check_state(if app_options.pol_sv {Checked} else {Unchecked});
    self.exp_sv_check.set_check_state(if app_options.exp_sv {Checked} else {Unchecked});
    self.flat_sv_check.set_check_state(if app_options.flat_sv {Checked} else {Unchecked});
    self.vol_check.set_check_state(if app_options.vol {Checked} else {Unchecked});
    self.hit_check.set_check_state(if app_options.hits {Checked} else {Unchecked});
    self.barline_check.set_check_state(if app_options.barlines {Checked} else {Unchecked});
    self.inh_check.set_check_state(if app_options.inh_lines {Checked} else {Unchecked});
    self.offset_text.set_text(&app_options.offset);
    self.buffer_text.set_text(&app_options.buffer);
    self.min_spacing_text.set_text(&app_options.min_spacing);
    self.pol_exp_text.set_text(&app_options.pol_exp);
    self.flat_sv_text.set_text(&app_options.flat_change);
    self.ign_bpm_check.set_check_state(if app_options.ignore_bpm {Checked} else {Unchecked});

    //validation on x/y
    if app_options.pos_x < 0 || app_options.pos_x > nwg::Monitor::width() - 300 {
      app_options.pos_x = cmp::max(0, nwg::Monitor::width() / 2 - 150);
    }
    if app_options.pos_y < 0 || app_options.pos_y > nwg::Monitor::height() {
      app_options.pos_y = cmp::max(0, nwg::Monitor::height() / 2 - 150);
    }

    self.window.set_position(app_options.pos_x, app_options.pos_y);
    self.window.set_size(app_options.width, app_options.height);
    self.resize_begin();
    self.resize_end();
    
    self.fill_out_filename();
    if self.in_filename.text().len() == 0 {
      self.apply_button.set_enabled(false);
    } else {
      self.load_file()
    }

    self.options.replace(app_options);

    Ok(())
  }

  //recheck and update all options
  fn update_config(&self) {
    let (x,y) = self.window.position();
    let (w,h) = self.window.size();

    let app_options = AppOptions{
      map: self.in_filename.text(),
      inh_times: self.inherited_text.text(),
      lin_sv: self.lin_sv_check.check_state() == Checked,
      pol_sv: self.pol_sv_check.check_state() == Checked,
      exp_sv: self.exp_sv_check.check_state() == Checked,
      flat_sv: self.flat_sv_check.check_state() == Checked,
      vol: self.vol_check.check_state() == Checked,
      hits: self.hit_check.check_state() == Checked,
      barlines: self.barline_check.check_state() == Checked,
      inh_lines: self.inh_check.check_state() == Checked,
      offset: self.offset_text.text(),
      buffer: self.buffer_text.text(),
      min_spacing: self.min_spacing_text.text(),
      pol_exp: self.pol_exp_text.text(),
      flat_change: self.flat_sv_text.text(),
      ignore_bpm: self.ign_bpm_check.check_state() == Checked,
      pos_x: x,
      pos_y: y,
      width: w,
      height: h,
      experimental: String::from(""),
    };

    self.options.replace(app_options);
  }

  //save current options to file
  fn save_config(&self) -> Result<()> {
    let mut out_string = String::new();
    let out_file_res = File::create(SVT_OPTIONS_FILE);

    //couldn't create config file, notify parent function
    if out_file_res.is_err() {
      return Err(anyhow!("Couldn't save config file!"));
    }

    let mut out_file = out_file_res.unwrap();

    out_string += &serde_json::to_string_pretty(&*self.options.borrow()).unwrap();
    let _ = write!(&mut out_file, "{}", out_string);
    Ok(())
  }

  //save current position
  fn resize_begin(&self) {
    let (x,y) = self.window.position();
    self.pos_x.replace(x);
    self.pos_y.replace(y);
  }

  //load the position from resize start and validate width/height
  fn resize_end(&self) {
    let (w,h) = self.window.size();

    let mut w_new = w;
    let mut h_new = h;

    if w != 300 {
      w_new = 300;
    }

    if h < 300 {
      h_new = 300;
    }

    if (w_new,h_new) != (w,h) {
      self.window.set_size(w_new, h_new);

      //reset position of window if resized
      let (x,y) = (self.pos_x.take(), self.pos_y.take());
      self.window.set_position(x,y);
    }
  }

  fn set_sv_mode(&self, ctrl: &nwg::CheckBox) {
    //ensure only one sv option is checked at a time
    if ctrl == &self.lin_sv_check {
      if self.lin_sv_check.check_state() == Checked {
        self.pol_sv_check.set_check_state(Unchecked);
        self.exp_sv_check.set_check_state(Unchecked);
        self.flat_sv_check.set_check_state(Unchecked);
      }
    } else if ctrl == &self.pol_sv_check {
      self.lin_sv_check.set_check_state(Unchecked);
      self.exp_sv_check.set_check_state(Unchecked);
      self.flat_sv_check.set_check_state(Unchecked);
    } else if ctrl == &self.exp_sv_check {
      if self.exp_sv_check.check_state() == Checked {
        self.lin_sv_check.set_check_state(Unchecked);
        self.pol_sv_check.set_check_state(Unchecked);
        self.flat_sv_check.set_check_state(Unchecked);
      }
    } else if ctrl == &self.flat_sv_check {
      if self.flat_sv_check.check_state() == Checked {
        self.lin_sv_check.set_check_state(Unchecked);
        self.pol_sv_check.set_check_state(Unchecked);
        self.exp_sv_check.set_check_state(Unchecked);
      }
    }

    //set visibility of mode specific options
    if self.pol_sv_check.check_state() == Checked {
      self.pol_exp_text.set_visible(true);
      self.pol_exp_label.set_visible(true);
    } else {
      self.pol_exp_text.set_visible(false);
      self.pol_exp_label.set_visible(false);
    }

    if self.flat_sv_check.check_state() == Checked {
      self.flat_sv_text.set_visible(true);
      self.flat_sv_label.set_visible(true);
      self.hit_check.set_enabled(false);
      self.barline_check.set_enabled(false);
      self.inh_check.set_enabled(false);
      self.ign_bpm_check.set_enabled(false);
    } else {
      self.flat_sv_text.set_visible(false);
      self.flat_sv_label.set_visible(false);
      self.hit_check.set_enabled(true);
      self.barline_check.set_enabled(true);
      self.inh_check.set_enabled(true);
      self.ign_bpm_check.set_enabled(true);
    }

    //set visibility of sv options
    if ctrl.check_state() == Checked {
      self.ign_bpm_check.set_visible(true);
    } else {
      self.ign_bpm_check.set_visible(false);
    }
  }
}