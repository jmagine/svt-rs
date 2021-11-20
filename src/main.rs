#![windows_subsystem = "windows"]

extern crate native_windows_gui as nwg;
extern crate native_windows_derive as nwd;

use nwd::NwgUi;
use nwg::NativeUi;
use std::{cell::RefCell};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::Path;

#[derive(Clone)]
pub struct MapObject {
  time: i32,
  class: i32,
  data: String,
}

#[derive(Default, NwgUi)]
pub struct SVT {
  //TODO make this pixel-perfect pretty
  #[nwg_control(size: (300, 300), position: (500, 500), title: "SVT", accept_files: true, flags: "WINDOW|VISIBLE")]
  #[nwg_events( OnWindowClose: [SVT::close_window], OnFileDrop: [SVT::drop_file(SELF, EVT_DATA)] )]
  window: nwg::Window,

  //timing point input
  #[nwg_control(text: "", size: (290, 50), position: (5,5), flags: "VISIBLE|AUTOVSCROLL|TAB_STOP")]
  inherited_text: nwg::TextBox,

  #[nwg_control(text: "Apply to:", size: (95, 20), position: (5, 60))]
  apply_label: nwg::Label,

  //toggles note changes
  #[nwg_control(text: "Hits", size: (95, 20), position: (5, 80), check_state: CheckBoxState::Checked)]
  hit_check: nwg::CheckBox,

  //toggles barline changes
  #[nwg_control(text: "Barlines", size: (95, 20), position: (5, 100), check_state: CheckBoxState::Checked)]
  barline_check: nwg::CheckBox,

  //toggles inh line changes
  #[nwg_control(text: "Inherited lines", size: (95, 20), position: (5, 120), check_state: CheckBoxState::Unchecked)]
  inh_check: nwg::CheckBox,

  //offset time
  #[nwg_control(text: "0", size: (20, 20), position: (105, 80))]
  offset_text: nwg::TextInput,

  #[nwg_control(text: "offset", size: (55, 20), position: (130, 82))]
  offset_label: nwg::Label,

  //buffer time
  #[nwg_control(text: "3", size: (20, 20), position: (105, 105))]
  buffer_text: nwg::TextInput,

  #[nwg_control(text: "buffer", size: (55, 20), position: (130, 107))]
  buffer_label: nwg::Label,

  //toggles inh line changes
  #[nwg_control(text: "Ignore end BPM", size: (105, 20), position: (185, 80), check_state: CheckBoxState::Unchecked)]
  eq_bpm_check: nwg::CheckBox,

  //input map filename
  #[nwg_control(text: "", size: (175, 23), position: (6, 186))]
  in_filename: nwg::TextInput,

  //select map button
  #[nwg_control(text: "Select .osu File", size: (110, 25), position: (185, 185))]
  #[nwg_events( OnButtonClick: [SVT::open_file] )]
  open_button: nwg::Button,

  //output map filename
  #[nwg_control(text: "", size: (175, 23), position: (6, 216))]
  out_filename: nwg::TextInput,

  //toggles preview
  #[nwg_control(text: "Preview changes", size: (110, 25), position: (185, 215), check_state: CheckBoxState::Checked)]
  #[nwg_events(OnButtonClick: [SVT::fill_out_filename])]
  preview_check: nwg::CheckBox,
  
  //place apply button near bottom
  #[nwg_control(text: "Apply", size: (290, 25), position: (5, 245))]
  #[nwg_events( OnButtonClick: [SVT::apply_sv] )]
  hello_button: nwg::Button,

  //place status bar at the very bottom
  #[nwg_control(text: "no map selected")]
  status: nwg::StatusBar,

  //open file dialog
  #[nwg_resource(title: "Open File", action: nwg::FileDialogAction::Open, filters: "osu(*.osu)")]
  file_dialog: nwg::FileDialog,

  all_objs: RefCell<Vec<MapObject>>,
  new_objs: RefCell<Vec<MapObject>>,
  //all_points: RefCell<Vec<String>>,
  //inh_points: RefCell<Vec<String>>,
  //inh_times: RefCell<Vec<i32>>,
  //hit_times: RefCell<Vec<i32>>,
  //bar_times: RefCell<Vec<i32>>,

  //new_points: RefCell<Vec<String>>,
}

impl SVT {
  fn apply_sv(&self) {
    self.load_file();

    self.new_objs.borrow_mut().clear();

    //take first 2 inherited lines and parse them
    let cmd = self.inherited_text.text();
    let mut lines = cmd.lines();
    let start_line = lines.next();
    let end_line = lines.next();

    match (start_line, end_line) {
      (Some(_), Some(_)) => {},
      _ => {
        println!("formatting issue: {}", cmd);
        return;
      },
    }

    let start_tokens: Vec<&str> = start_line.unwrap().split(",").collect();
    let end_tokens: Vec<&str> = end_line.unwrap().split(",").collect();

    if start_tokens.len() != 8 || end_tokens.len() != 8 {
      println!("formatting issue: {}", cmd);
      return;
    }

    let t_offset = self.offset_text.text().parse::<i32>();
    let t_buffer = self.buffer_text.text().parse::<i32>();

    let start_time = start_tokens[0].parse::<i32>();
    let start_bl = start_tokens[1].parse::<f32>();
    let start_vol = start_tokens[5].parse::<i32>();

    let end_time = end_tokens[0].parse::<i32>();
    let end_bl = end_tokens[1].parse::<f32>();
    let end_vol = end_tokens[5].parse::<i32>();

    match (t_offset, t_buffer, start_time, start_bl, start_vol, end_time, end_bl, end_vol) {
      (Ok(t_off), Ok(t_buf), Ok(s_t), Ok(s_b), Ok(s_v), Ok(e_t), Ok(e_b), Ok(e_v)) => {
        //all tokens valid

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
              end_bpm = obj_tokens[1].parse::<f32>().unwrap();
            }
          }
        }

        //convert beatlength values to sv values
        //sv = -100.0 / beatlength
        //raw_sv = sv * bpm
        let s_sv_raw = -100.0 / s_b * start_bpm;
        let e_sv_raw = if self.eq_bpm_check.check_state() == nwg::CheckBoxState::Checked {
          -100.0 / e_b * start_bpm
        } else {
          -100.0 / e_b * end_bpm
        };
        //let s_sv_raw = -100.0 / s_b * start_bpm;
        //let e_sv_raw = -100.0 / e_b * end_bpm;

        //TODO equalize sv based on bpm, which may change depending on uninherited line positions
        //let s_seq = s_s * bpm;
        //let e_seq = e_s * bpm;

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
          //println!("[apply] {} {} {}", obj.time, obj.class, obj.data);
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
            let new_sv = s_sv_raw + (obj_time - s_t) as f32 * sv_per_ms;
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
      },
      _ => {
        println!("issue: {}", cmd);
        return;
      },
    }

    //merge new points into old ones - delete old point if new one is identical
    let mut out_objs: Vec<MapObject> = Vec::new();
    out_objs.extend(self.new_objs.borrow().iter().cloned());
    for obj in self.all_objs.borrow().iter() {
      //uninherited/inherited lines
      if obj.class == 0 || obj.class == 1 {
        out_objs.push(obj.clone());
      }
    }
    //out_objs.extend(self.all_objs.borrow().iter().cloned());
    out_objs.sort_by_key(|k| (k.time, k.data.split(",").nth(6).unwrap().parse::<i32>().unwrap()));
    out_objs.dedup_by_key(|k| (k.time, k.data.split(",").nth(6).unwrap().parse::<i32>().unwrap()));

    //all output is same except for timing points section

    //write out new file
    let in_filename = self.in_filename.text();
    let out_filename = self.out_filename.text();

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
  
  fn close_window(&self) {
    nwg::stop_thread_dispatch();
  }

  fn open_file(&self) {
    if let Ok(d) = env::current_dir() {
      if let Some(d) = d.to_str() {
        self.file_dialog.set_default_folder(d).expect("Failed to set default folder.");
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

  pub fn drop_file(&self, data: &nwg::EventData) {
    self.in_filename.set_text(&data.on_file_drop().files().pop().unwrap());
    self.fill_out_filename();
    self.load_file();
  }

  pub fn fill_out_filename(&self) {
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

  // load and parse .osu file line by line
  pub fn load_file(&self) {
    let filename = self.in_filename.text();

    // determine filename and extension
    let ext = Path::new(&filename).extension();

    let folder = String::from(Path::new(&filename).parent().unwrap().to_str().unwrap());
    println!("[load] folder: {}", folder);

    // skip any file that is not .osu
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
    //self.inh_points = Vec::new();
    //self.inh_times = Vec::new();
    //self.hit_times = Vec::new();

    self.all_objs.borrow_mut().clear();
    //self.inh_points.borrow_mut().clear();
    //self.hit_times.borrow_mut().clear();
    //self.bar_times.borrow_mut().clear();

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
                //println!("[load] timing {}", s);
                //TODO do barlines when processing sv section and then process the hit objects vec + barlines vec after filereading is complete
                
                
                //if let Ok(time) = s.split(",").nth(0).unwrap().parse::<i32>() {
                //  println!("[load] sv {}", time);
                  //self.inh_times.borrow_mut().push(time);
                //}

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
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(full_path: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
  let file = File::open(full_path)?;
  Ok(io::BufReader::new(file).lines())
}

fn main() {
  nwg::init().expect("Failed to init Native Windows GUI");

  // use Segoe UI with 16 size as default font
  let mut font = nwg::Font::default();

  nwg::Font::builder()
    .family("Segoe UI")
    .size(16)
    .build(&mut font).ok();
  nwg::Font::set_global_default(Some(font));

  let _app = SVT::build_ui(Default::default()).expect("Failed to build UI");

  nwg::dispatch_thread_events();
}