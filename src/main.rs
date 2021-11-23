#![windows_subsystem = "windows"]

extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;

use nwd::NwgUi;
use nwg::NativeUi;
use std::{cell::RefCell};
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::cmp;

#[derive(Clone)]
pub struct MapObject {
  time: i32,
  class: i32,
  data: String,
}

#[derive(Default, NwgUi)]
pub struct SVT {
  //TODO make this pixel-perfect pretty
  #[nwg_control(size: (300, 300), position: (cmp::max(0, nwg::Monitor::width() / 2 - 150), cmp::max(0, nwg::Monitor::height() / 2 - 150)), title: "SVT", accept_files: true, flags: "WINDOW|VISIBLE|MINIMIZE_BOX")]
  #[nwg_events( OnWindowClose: [SVT::close_window], OnFileDrop: [SVT::drop_file(SELF, EVT_DATA)] )]
  window: nwg::Window,

  //timing point input
  #[nwg_control(text: "", size: (290, 50), position: (5,5), flags: "VISIBLE|AUTOVSCROLL|TAB_STOP")]
  inherited_text: nwg::TextBox,

  //apply
  #[nwg_control(size: (100, 85), position: (5, 60))]
  apply_frame: nwg::Frame,

  #[nwg_control(text: "Apply to:", size: (95, 20), position: (2, 0), parent: apply_frame)]
  apply_label: nwg::Label,

  //toggles note changes
  #[nwg_control(text: "Hits", size: (95, 20), position: (2, 20), check_state: CheckBoxState::Checked, parent: apply_frame)]
  hit_check: nwg::CheckBox,

  //toggles barline changes
  #[nwg_control(text: "Barlines", size: (95, 20), position: (2, 40), check_state: CheckBoxState::Checked, parent: apply_frame)]
  barline_check: nwg::CheckBox,

  //toggles inh line changes
  #[nwg_control(text: "Inherited lines", size: (95, 20), position: (2, 60), check_state: CheckBoxState::Unchecked, parent: apply_frame)]
  inh_check: nwg::CheckBox,

  //options
  #[nwg_control(size: (185, 85), position: (110, 60))]
  options_frame: nwg::Frame,

  #[nwg_control(text: "Options:", size: (95, 20), position: (2, 0), parent: options_frame)]
  options_label: nwg::Label,

  //offset time
  #[nwg_control(text: "0", size: (19, 19), position: (2, 20), parent: options_frame)]
  offset_text: nwg::TextInput,

  #[nwg_control(text: "Offset", size: (45, 20), position: (25, 22), parent: options_frame)]
  offset_label: nwg::Label,

  //buffer time
  #[nwg_control(text: "3", size: (19, 19), position: (2, 40), parent: options_frame)]
  buffer_text: nwg::TextInput,

  #[nwg_control(text: "Buffer", size: (45, 20), position: (25, 42), parent: options_frame)]
  buffer_label: nwg::Label,

  //exponential factor
  #[nwg_control(text: "0.5", size: (19, 19), position: (2, 60), parent: options_frame)]
  exponent_text: nwg::TextInput,

  #[nwg_control(text: "Exp.", size: (45, 20), position: (25, 62), parent: options_frame)]
  exponent_label: nwg::Label,

  //toggles end line/start line BPM
  #[nwg_control(text: "Ignore end BPM", size: (105, 20), position: (75, 20), check_state: CheckBoxState::Unchecked, parent: options_frame)]
  eq_bpm_check: nwg::CheckBox,

  //toggles exponential mode
  #[nwg_control(text: "Exponential SV", size: (105, 20), position: (75, 40), check_state: CheckBoxState::Unchecked, parent: options_frame)]
  exponential_check: nwg::CheckBox,

  //input map filename
  #[nwg_control(text: "", size: (185, 23), position: (110, 186))]
  in_filename: nwg::TextInput,

  //select map button
  #[nwg_control(text: "Select Mapfile", size: (102, 25), position: (4, 185))]
  #[nwg_events( OnButtonClick: [SVT::open_file_browser] )]
  open_button: nwg::Button,

  //output map filename
  #[nwg_control(text: "", size: (185, 23), position: (110, 216))]
  out_filename: nwg::TextInput,

  //toggles preview
  #[nwg_control(text: "Preview Output", size: (102, 25), position: (5, 215), check_state: CheckBoxState::Checked)]
  #[nwg_events(OnButtonClick: [SVT::fill_out_filename])]
  preview_check: nwg::CheckBox,
  
  //place apply button near bottom
  #[nwg_control(text: "Apply", size: (292, 25), position: (4, 245))]
  #[nwg_events( OnButtonClick: [SVT::apply_changes] )]
  hello_button: nwg::Button,

  //place status bar at the very bottom
  #[nwg_control(text: "no map selected")]
  status: nwg::StatusBar,

  //open file dialog
  #[nwg_resource(title: "Open File", action: nwg::FileDialogAction::Open, filters: "osu(*.osu)")]
  file_dialog: nwg::FileDialog,

  all_objs: RefCell<Vec<MapObject>>,
  new_objs: RefCell<Vec<MapObject>>,
}

impl SVT {
  fn apply_changes(&self) {
    //refresh file before doing anything
    self.load_file();

    //clear any previously applied points
    self.new_objs.borrow_mut().clear();

    let cmd = self.inherited_text.text();
    let mut lines = cmd.lines();
    let mut start_line;
    let mut end_line;

    //process 2 valid lines at a time until no lines left
    loop {
      //skip empty lines
      //TODO could also attempt to do more data validation at this step
      start_line = lines.next();
      while start_line == Some("") {start_line = lines.next();}

      end_line = lines.next();
      while end_line == Some("") {end_line = lines.next();}

      if let (Some(start_l), Some(end_l)) = (start_line, end_line) {
        self.apply_timing(start_l, end_l);
      } else {
        println!("[apply] no more lines");
        break;
      }
    }

    //don't write anything if no new objects
    if self.new_objs.borrow().len() == 0 {
      println!("[apply] no new objects, early termination");
      return;
    }

    //merge new points into old ones - delete old point if new one is identical
    self.write_output_points();
  }

  fn apply_timing(&self, start_line: &str, end_line: &str) {
    let start_tokens: Vec<&str> = start_line.split(",").collect();
    let end_tokens: Vec<&str> = end_line.split(",").collect();

    if start_tokens.len() != 8 || end_tokens.len() != 8 {
      println!("formatting issue:\n{}\n{}", start_line, end_line);
      return;
    }

    let exponent = self.exponent_text.text().parse::<f32>();

    let t_offset = self.offset_text.text().parse::<i32>();
    let t_buffer = self.buffer_text.text().parse::<i32>();

    let start_time = start_tokens[0].parse::<i32>();
    let start_bl = start_tokens[1].parse::<f32>();
    let start_vol = start_tokens[5].parse::<i32>();

    let end_time = end_tokens[0].parse::<i32>();
    let end_bl = end_tokens[1].parse::<f32>();
    let end_vol = end_tokens[5].parse::<i32>();

    //all token validation
    if let (Ok(t_off), Ok(t_buf), Ok(s_t), Ok(s_b), Ok(s_v), Ok(e_t), Ok(e_b), Ok(e_v), Ok(exp)) = (t_offset, t_buffer, start_time, start_bl, start_vol, end_time, end_bl, end_vol, exponent) {
      //determine bpm at starting point
      let mut start_bpm = 160.0;
      let mut end_bpm = 160.0;
      for obj in self.all_objs.borrow().iter() {
        if obj.class == 0 {
          let obj_tokens: Vec<&str> = obj.data.split(",").collect();
          if obj.time < s_t {
            start_bpm = 60000.0 / obj_tokens[1].parse::<f32>().unwrap();
          }
          if obj.time < e_t {
            end_bpm = 60000.0 /  obj_tokens[1].parse::<f32>().unwrap();
          }
        }
      }

      //convert beatlength values to sv values
      let s_sv_raw = -100.0 / s_b * start_bpm;
      let e_sv_raw = if self.eq_bpm_check.check_state() == nwg::CheckBoxState::Checked {
        -100.0 / e_b * start_bpm
      } else {
        -100.0 / e_b * end_bpm
      };

      //debug print
      println!("[apply] t:{}->{} raw sv:{}->{} vol:{}->{}", s_t, e_t, s_sv_raw, e_sv_raw, s_v, e_v);

      //validation on input values
      if s_t > e_t {println!("[apply] start time should be less than end time"); return;}
      if s_sv_raw <= 0.0 || e_sv_raw <= 0.0 {println!("[apply] sv values should be positive (neg beatlength values"); return;} 
      if s_v < 0 || s_v > 100 || e_v < 0 || e_v > 100 {println!("[apply] volumes should be within [0, 100]"); return;}
      
      //compute change per time tick
      let t_diff = e_t - s_t;
      let sv_diff = e_sv_raw - s_sv_raw;
      let v_diff = e_v - s_v;
      let sv_per_ms = sv_diff / t_diff as f32;
      let v_per_ms = v_diff as f32 / t_diff as f32;

      let mut bpm = 160.0;
      let mut meter = 4;
      let mut sample_set = 0;
      let mut sample_index = 0;
      let mut effects = 0;

      for obj in self.all_objs.borrow().iter() {
        let obj_tokens: Vec<&str> = obj.data.split(",").collect();
        //handle uninherited lines differently
        if obj.class == 0 {
          bpm = 60000.0 / obj_tokens[1].parse::<f32>().unwrap();
          meter = obj_tokens[2].parse::<i32>().unwrap();
          sample_set = obj_tokens[3].parse::<i32>().unwrap();
          sample_index = obj_tokens[4].parse::<i32>().unwrap();
          effects = obj_tokens[7].parse::<i32>().unwrap();
          continue;
        }

        //perform general calculations here for inher, barlines, hitobjects
        let obj_time = obj.time;
        if obj_time >= s_t - t_buf && obj_time <= e_t + t_buf {
          let new_t = obj_time + t_off;
          let new_sv = if self.exponential_check.check_state() == nwg::CheckBoxState::Checked {
            //exponential
            s_sv_raw + sv_diff * f32::powf((obj_time - s_t) as f32 / t_diff as f32, exp)
          } else {
            //linear
            s_sv_raw + (obj_time - s_t) as f32 * sv_per_ms
          };

          let new_b = -100.0 / (new_sv / bpm);
          let new_v = (s_v as f32 + (obj_time - s_t) as f32 * v_per_ms) as u32;
          let new_point = format!("{},{},{},{},{},{},{},{}", new_t, new_b, meter, sample_set, sample_index, new_v, 0, effects);
        

          match obj.class {
            0 => {println!("[apply] shouldn't get here, class 0");}, //uninherited line
            1 => {
              //inherited line
              if self.inh_check.check_state() == nwg::CheckBoxState::Checked {
                println!("[new] inh {}", new_point);
                self.new_objs.borrow_mut().push(MapObject{time: new_t, class: 4, data: new_point});

                sample_set = obj_tokens[3].parse::<i32>().unwrap();
                sample_index = obj_tokens[4].parse::<i32>().unwrap();
                effects = obj_tokens[7].parse::<i32>().unwrap();
              }
            },
            2 => {
              //barline
              if self.barline_check.check_state() == nwg::CheckBoxState::Checked {
                println!("[new] bar {}", new_point);
                self.new_objs.borrow_mut().push(MapObject{time: new_t, class: 4, data: new_point});
              }
            },
            3 => {
              //hitobject
              if self.hit_check.check_state() == nwg::CheckBoxState::Checked {
                println!("[new] hit {}", new_point);
                self.new_objs.borrow_mut().push(MapObject{time: new_t, class: 4, data: new_point});
              }
            },
            _ => {
              println!("[apply] unknown class {}", obj.class);
            }
          }
        }
      }
    } else {
      println!("[apply] issue:\n{}\n{}", start_line, end_line);
      return;
    }
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
    if self.preview_check.check_state() == nwg::CheckBoxState::Checked {
      let in_filename = &self.in_filename.text();

      //prevent paths without parents or filenames from crashing
      let folder = Path::new(in_filename).parent();
      let name_osu = Path::new(in_filename).file_name();

      match (folder, name_osu) {
        (Some(folder), Some(name_osu)) => {
          self.out_filename.set_text(&format!("{}/{}[{}].osu", folder.to_str().unwrap(), name_osu.to_str().unwrap().split("[").nth(0).unwrap(), "preview"));
        },
        _ => {
          println!("[pre] path invalid");
        },
      }
    } else {
      self.out_filename.set_text(&self.in_filename.text());
    }
  }

  //load and parse .osu file line by line
  fn load_file(&self) {
    let filename = self.in_filename.text();

    if filename.len() == 0 {
      println!("[load] empty filename");
      return;
    }

    //determine filename and extension
    let ext = Path::new(&filename).extension();

    let folder = String::from(Path::new(&filename).parent().unwrap().to_str().unwrap());
    println!("[load] folder: {}", folder);

    //skip any file that is not .osu
    match ext {
      Some(str) => if str.to_str() != Some("osu") {
        println!("[load] incorrect file type: .{}", str.to_str().unwrap());
        return},
      None => {
        return
      },
    }

    println!("[load] loading {}", Path::new(&filename).file_name().unwrap().to_str().unwrap());

    let mut bool_timing = false;
    let mut bool_hit = false;

    //TODO figure out better way to init this
    let mut bar_time: f32 = 100000.0;
    let mut bar_inc: f32 = 100000.0;

    self.all_objs.borrow_mut().clear();

    // read file line by line
    if let Ok(lines) = read_lines(&filename) {
      for line in lines {
        if let Ok(s) = line {
          // we only care about the TimingPoints and HitObjects headers
          match s.as_str() {
            "[General]" | "[Editor]" | "[Metadata]" | "[Difficulty]" | "[Events]" | "[Colours]" => {
              bool_timing = false;
              bool_hit = false;
            },
            "[TimingPoints]" => {
              bool_timing = true;
              bool_hit = false;
              println!("[load] found [TimingPoints], reading");
            },
            "[HitObjects]" => {
              bool_timing = false;
              bool_hit = true;
              println!("[load] found [HitObjects], reading");
            },
            _ => {
              if bool_timing {
                let s_tokens: Vec<&str> = s.split(",").collect();
                if s_tokens.len() != 8 {
                  continue;
                }

                let time = s_tokens[0].parse::<i32>();
                let beatlength = s_tokens[1].parse::<f32>();
                let meter = s_tokens[2].parse::<i32>();
                let uninherited = s_tokens[6].parse::<i32>();
                let effect = s_tokens[7].parse::<i32>();
                
                match (time, beatlength, meter, uninherited, effect) {
                  (Ok(t), Ok(bl), Ok(m), Ok(uninh), Ok(eff)) => {
                    //add barlines since last timing point
                    while bar_time + bar_inc < t as f32 {
                      bar_time += bar_inc;
                      self.all_objs.borrow_mut().push(MapObject{time: bar_time as i32, class: 2, data: String::from("")});
                    }

                    if uninh == 1 {
                      //uninherited point
                      self.all_objs.borrow_mut().push(MapObject{time: t, class: 0, data: s.clone()});

                      //only skip barline if effect bit 3 is set
                      if eff & 8 != 8 {
                        self.all_objs.borrow_mut().push(MapObject{time: t, class: 2, data: String::from("")});
                      }

                      //set barline counter based on uninherited point
                      bar_time = t as f32;
                      bar_inc = bl * m as f32;
                    } else if uninh == 0 {
                      //inherited point
                      self.all_objs.borrow_mut().push(MapObject{time: t, class: 1, data: s.clone()});
                    }
                  },
                  _ => {
                    println!("[load] issue {}", s);
                    return;
                  },
                }
              } else if bool_hit {
                if let Ok(hit_time) = s.split(",").nth(2).unwrap().parse::<i32>() {
                  self.all_objs.borrow_mut().push(MapObject{time: hit_time, class: 3, data: String::from("")});
                }
              } else {
                continue;
              }
            },
          }
        }
      }
    }
    self.all_objs.borrow_mut().sort_by_key(|k| k.time);
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

  fn write_output_points(&self) {
    //sort new objects in chronological order
    //TODO inherited vs uninherited should not apply here, these are the new points
    self.new_objs.borrow_mut().sort_by_key(|k| (k.time, k.data.split(",").nth(6).unwrap().parse::<i32>().unwrap()));

    //build up a vector with all old and new points sorted in chronological, then uninherited > inherited order
    let mut out_objs: Vec<MapObject> = Vec::new();
    out_objs.extend(self.new_objs.borrow().iter().cloned());
    for obj in self.all_objs.borrow().iter() {
      //uninherited/inherited lines
      if obj.class == 0 || obj.class == 1 {
        out_objs.push(obj.clone());
      }
    }
    out_objs.sort_by_key(|k| (k.time, k.data.split(",").nth(6).unwrap().parse::<i32>().unwrap()));
    out_objs.dedup_by_key(|k| (k.time, k.data.split(",").nth(6).unwrap().parse::<i32>().unwrap()));

    //write out new file
    let in_filename = self.in_filename.text();
    let out_filename = self.out_filename.text();

    //make backup before writing file, don't write without backing up
    if let Err(e) = fs::copy(&in_filename, "backup.osu") {
      println!("[backup] error backing up file {}", e);
      return;
    }

    // read file line by line
    let mut out_string = String::new();
    let mut bool_timing = false;
    if let Ok(lines) = read_lines(&in_filename) {
      for line in lines {
        if let Ok(s) = line {
          // we want everything except timingpoints lines
          match s.as_str() {
            "[General]" | "[Editor]" | "[Metadata]" | "[Difficulty]" | "[Events]" | "[Colours]" | "[HitObjects]" => {
              bool_timing = false;
              out_string += &s;
              out_string += "\n";
            },
            "[TimingPoints]" => {
              bool_timing = true;
              out_string += &s;
              out_string += "\n";
              for out_obj in out_objs.iter() {
                out_string += &out_obj.data;
                out_string += "\n";
              }
              out_string += "\n";
            },
            _ => {
              let s_tokens: Vec<&str> = s.split(":").collect();
              if self.preview_check.check_state() == nwg::CheckBoxState::Checked && s_tokens.len() == 2 && s_tokens[0] == "Version" {
                out_string += "Version:preview\n";
              } else if !bool_timing {
                out_string += &s;
                out_string += "\n";
              }
            },
          }
        }
      }
    }
    let mut out_file = File::create(out_filename).unwrap();
    let _ = write!(&mut out_file, "{}", out_string);
  }
}

fn read_lines<P>(full_path: P) -> io::Result<io::Lines<io::BufReader<File>>> where P: AsRef<Path>, {
  let file = File::open(full_path)?;
  Ok(io::BufReader::new(file).lines())
}

fn main() {
  let icon_bytes = include_bytes!("../assets/svt.ico");
  nwg::init().expect("[main] failed to init nwg");

  //use Segoe UI with 16 size as default font
  let mut font = nwg::Font::default();

  nwg::Font::builder()
    .family("Segoe UI")
    .size(16)
    .build(&mut font).ok();
  nwg::Font::set_global_default(Some(font));

  //set icon on taskbar and on window top left
  let mut icon = nwg::Icon::default();
  let _res_ = nwg::Icon::builder()
    .source_bin(Some(icon_bytes))
    .strict(true)
    .build(&mut icon);

  let app = SVT::build_ui(Default::default()).expect("[main] failed to build UI");
  app.window.set_icon(Some(&icon));

  nwg::dispatch_thread_events();
}