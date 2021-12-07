extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;
use nwg::CheckBoxState::{Checked, Unchecked};
use nwd::NwgUi;

use anyhow::Result;
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
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppOptions {
  map: String,
  sv: bool,
  vol: bool,
  hits: bool,
  barlines: bool,
  inh_lines: bool,
  offset: String,
  buffer: String,
  exp: String,
  ignore_bpm: bool,
  exp_sv: bool,
  experimental: String,
}

#[derive(Default, NwgUi)]
pub struct UI {
  #[nwg_control(size: (300, 300), position: (cmp::max(0, nwg::Monitor::width() / 2 - 150), cmp::max(0, nwg::Monitor::height() / 2 - 150)), title: "SVT", accept_files: true, flags: "WINDOW|VISIBLE|MINIMIZE_BOX")]
  #[nwg_events( OnWindowClose: [UI::close_window], OnFileDrop: [UI::drop_file(SELF, EVT_DATA)] )]
  pub window: nwg::Window,

  //timing point input
  #[nwg_control(text: "", size: (290, 80), position: (5,5), flags: "VISIBLE|AUTOVSCROLL|TAB_STOP")]
  pub inherited_text: nwg::TextBox,

  //outline around the apply controls
  #[nwg_control(size: (50, 85), position: (5, 90))]
  pub apply_frame: nwg::Frame,

  #[nwg_control(text: "Apply:", size: (45, 20), position: (2, 0), parent: apply_frame)]
  pub apply_label: nwg::Label,

  //toggles sv changes
  #[nwg_control(text: "SV", size: (95, 20), position: (2, 20), check_state: Checked, parent: apply_frame)]
  pub sv_check: nwg::CheckBox,

  //toggles vol changes
  #[nwg_control(text: "Vol", size: (95, 20), position: (2, 40), check_state: Checked, parent: apply_frame)]
  pub vol_check: nwg::CheckBox,

  //outline around the apply to controls
  #[nwg_control(size: (70, 85), position: (54, 90))]
  pub apply_to_frame: nwg::Frame,

  #[nwg_control(text: "To:", size: (65, 20), position: (2, 0), parent: apply_to_frame)]
  pub apply_to_label: nwg::Label,

  //toggles note changes
  #[nwg_control(text: "Hits", size: (95, 20), position: (2, 20), check_state: Checked, parent: apply_to_frame)]
  pub hit_check: nwg::CheckBox,

  //toggles barline changes
  #[nwg_control(text: "Barlines", size: (95, 20), position: (2, 40), check_state: Checked, parent: apply_to_frame)]
  pub barline_check: nwg::CheckBox,

  //toggles inh line changes
  #[nwg_control(text: "Inh. lines", size: (95, 20), position: (2, 60), check_state: Unchecked, parent: apply_to_frame)]
  pub inh_check: nwg::CheckBox,

  //outline around advanced controls
  #[nwg_control(size: (165, 85), position: (130, 90))]
  pub options_frame: nwg::Frame,

  #[nwg_control(text: "Advanced Options:", size: (195, 20), position: (2, 0), parent: options_frame)]
  pub options_label: nwg::Label,

  //offset time
  #[nwg_control(text: "0", size: (19, 19), position: (2, 20), parent: options_frame)]
  pub offset_text: nwg::TextInput,

  #[nwg_control(text: "Offset", size: (45, 20), position: (25, 22), parent: options_frame)]
  pub offset_label: nwg::Label,

  //buffer time
  #[nwg_control(text: "3", size: (19, 19), position: (2, 40), parent: options_frame)]
  pub buffer_text: nwg::TextInput,

  #[nwg_control(text: "Buffer", size: (45, 20), position: (25, 42), parent: options_frame)]
  pub buffer_label: nwg::Label,

  //exponential factor
  #[nwg_control(text: "0.5", size: (19, 19), position: (2, 60), parent: options_frame)]
  pub exponent_text: nwg::TextInput,

  #[nwg_control(text: "Exp.", size: (45, 20), position: (25, 62), parent: options_frame)]
  pub exponent_label: nwg::Label,

  //toggles end line/start line BPM
  #[nwg_control(text: "Ignore BPM", size: (105, 20), position: (75, 20), check_state: Unchecked, parent: options_frame)]
  pub eq_bpm_check: nwg::CheckBox,

  //toggles exponential mode
  #[nwg_control(text: "Exp. SV", size: (105, 20), position: (75, 60), check_state: Unchecked, parent: options_frame)]
  pub exponential_check: nwg::CheckBox,

  //select map button
  #[nwg_control(text: "Select Map", size: (87, 25), position: (4, 185))]
  #[nwg_events( OnButtonClick: [UI::open_file_browser] )]
  pub open_button: nwg::Button,

  //input map filename
  #[nwg_control(text: "", size: (200, 23), position: (95, 186), flags: "VISIBLE|DISABLED")]
  pub in_filename: nwg::TextInput,

  //toggles preview
  #[nwg_control(text: "Preview Diff", size: (87, 25), position: (5, 215), check_state: Unchecked)]
  #[nwg_events(OnButtonClick: [UI::fill_out_filename])]
  pub preview_check: nwg::CheckBox,
  
  //output map filename
  #[nwg_control(text: "", size: (200, 23), position: (95, 216))]
  pub out_filename: nwg::TextInput,
  
  //place apply button near bottom
  #[nwg_control(text: "Apply", size: (242, 25), position: (4, 245), flags: "VISIBLE|DISABLED")]
  #[nwg_events( OnButtonClick: [UI::apply_changes] )]
  pub apply_button: nwg::Button,

  //place undo/redo button near bottom
  #[nwg_control(text: "Undo", size: (45, 25), position: (251, 245), flags: "VISIBLE|DISABLED")]
  #[nwg_events( OnButtonClick: [UI::undo] )]
  pub undo_button: nwg::Button,

  //place status bar at the very bottom
  #[nwg_control(text: "[map] no map selected (Select Mapfile or drag one in)")]
  pub status: nwg::StatusBar,

  //open file dialog
  #[nwg_resource(title: "Open File", action: nwg::FileDialogAction::Open, filters: "osu(*.osu)")]
  pub file_dialog: nwg::FileDialog,

  pub svt: RefCell<svt::SVT>,
}

impl UI {
  pub fn init(&self) {
    //set icon on taskbar and on window top left
    let icon_bytes = include_bytes!("../assets/svt.ico");
    let mut icon = nwg::Icon::default();
    let _res_ = nwg::Icon::builder()
      .source_bin(Some(icon_bytes))
      .strict(true)
      .build(&mut icon);
    self.window.set_icon(Some(&icon));

    //load config and sset apply button accordingly
    if self.load_config().is_err() {
      println!("[load] couldn't load config properly");
      self.apply_button.set_enabled(false);
    }

    //always disable undo button by default
    self.undo_button.set_enabled(false);
  }

  fn apply_changes(&self) {
    //refresh file before doing anything
    self.load_file();

    //[debug] print out all objects in their current order
    //self.svt.borrow().print_debug();

    let cmd = self.inherited_text.text();
    let mut lines = cmd.split_whitespace();
    let mut start_line;
    let mut end_line;

    //process 2 valid lines at a time until no lines left
    loop {
      start_line = lines.next();
      end_line = lines.next();
      if let (Some(start_l), Some(end_l)) = (start_line, end_line) {
        if let Err(err) = self.svt.borrow_mut().apply_timing(start_l, end_l, self) {
          //if error is encountered, stop applying and update status bar
          println!("[apply] error applying timing {}->{}", start_l, end_l);
          self.status.set_text(0, &err.to_string());
          return;
        }
      } else {
        println!("[apply] no more lines");
        break;
      }
    }

    //merge new points into old ones - delete old point if new one is identical
    if let Err(err) = self.svt.borrow_mut().write_output_points(self.in_filename.text(), self.out_filename.text(), self.preview_check.check_state() == Checked) {
      println!("[apply] error writing output");
      self.status.set_text(0, &err.to_string());
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
    self.status.set_text(0, &format!("[apply] {} changes applied", self.svt.borrow().new_objs.len()));
  }
  
  fn close_window(&self) {
    nwg::stop_thread_dispatch();
  }

  fn drop_file(&self, data: &nwg::EventData) {
    self.in_filename.set_text(&data.on_file_drop().files().pop().unwrap());
    self.fill_out_filename();
    self.load_file();
  }

  fn fill_out_filename(&self) {
    if self.preview_check.check_state() == Checked {
      let in_filename = &self.in_filename.text();

      //prevent paths without parents or filenames from crashing
      let folder = Path::new(in_filename).parent();
      let name_osu = Path::new(in_filename).file_name();

      //TODO check path is valid maybe?
      if let (Some(folder), Some(name_osu)) = (folder, name_osu) {
        self.out_filename.set_text(&format!("{}/{}[{}].osu", folder.to_str().unwrap(), name_osu.to_str().unwrap().split("[").nth(0).unwrap(), "preview"));
      } else {
        println!("[pre] path invalid");
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

    let folder = String::from(Path::new(&filename).parent().unwrap().to_str().unwrap());
    println!("[load] folder: {}", folder);

    println!("[load] loading {}", Path::new(&filename).file_name().unwrap().to_str().unwrap());
    self.svt.borrow_mut().load_osu(&filename);

    if self.save_config().is_err() {
      self.status.set_text(0, &format!("[apply] couldn't save config"));
      return;
    }

    self.apply_button.set_enabled(true);

    self.status.set_text(0, &format!("editing {}", Path::new(&filename).file_name().unwrap().to_str().unwrap()));
  }

  fn open_file_browser(&self) {
    if let Ok(d) = env::current_dir() {
      if let Some(d) = d.to_str() {
        self.file_dialog.set_default_folder(d).expect("[brow] failed to set default folder");
      }
    }
  
    if self.file_dialog.run(Some(&self.window)) {
      self.in_filename.set_text("");
      if let Ok(directory) = self.file_dialog.get_selected_item() {
        let dir = directory.into_string().unwrap();
        self.in_filename.set_text(&dir);
        self.fill_out_filename();
        self.load_file();
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
    let app_options_string = fs::read_to_string("svt_config.txt")?;    
    let app_options = serde_json::from_str(&app_options_string).unwrap_or(AppOptions{offset: String::from("0"), buffer: String::from("3"), exp: String::from("0.5"), ..Default::default()});

    self.in_filename.set_text(&app_options.map);
    self.sv_check.set_check_state(if app_options.sv {Checked} else {Unchecked});
    self.vol_check.set_check_state(if app_options.vol {Checked} else {Unchecked});
    self.hit_check.set_check_state(if app_options.hits {Checked} else {Unchecked});
    self.barline_check.set_check_state(if app_options.barlines {Checked} else {Unchecked});
    self.inh_check.set_check_state(if app_options.inh_lines {Checked} else {Unchecked});
    self.offset_text.set_text(&app_options.offset);
    self.buffer_text.set_text(&app_options.buffer);
    self.exponent_text.set_text(&app_options.exp);
    self.eq_bpm_check.set_check_state(if app_options.ignore_bpm {Checked} else {Unchecked});
    self.exponential_check.set_check_state(if app_options.exp_sv {Checked} else {Unchecked});
    
    self.fill_out_filename();
    if self.in_filename.text().len() == 0 {
      self.apply_button.set_enabled(false);
    } else {
      self.load_file()
    }

    Ok(())
  }

  fn save_config(&self) -> Result<()> {
    let mut out_string = String::new();
    let mut out_file = File::create("svt_config.txt").unwrap();

    let app_options = AppOptions{
      map: self.in_filename.text(),
      sv: self.sv_check.check_state() == Checked,
      vol: self.vol_check.check_state() == Checked,
      hits: self.hit_check.check_state() == Checked,
      barlines: self.barline_check.check_state() == Checked,
      inh_lines: self.inh_check.check_state() == Checked,
      offset: self.offset_text.text(),
      buffer: self.buffer_text.text(),
      exp: self.exponent_text.text(),
      ignore_bpm: self.eq_bpm_check.check_state() == Checked,
      exp_sv: self.exponential_check.check_state() == Checked,
      experimental: String::from(""),
    };

    out_string += &serde_json::to_string(&app_options).unwrap();

    let _ = write!(&mut out_file, "{}", out_string);
    Ok(())
  }
}