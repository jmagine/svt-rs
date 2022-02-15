#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

extern crate native_windows_gui as nwg;
use nwg::NativeUi;

//extern crate backtrace;
//use backtrace::Backtrace;
//use std::panic;

mod ui;
mod svt;

fn main() {
  /*
  panic::set_hook(Box::new(move |info| {
    // Docs say info.location() allways returns some atm
    if let Some(location) = info.location() {
      println!("Panic at {}:{}", location.file(), location.line());
  }
  
  let trace = Backtrace::new();
  
  for frame in trace.frames().iter() {
      for symbol in frame.symbols().iter() {
          // TODO These unwraps are not safe, and will cause a second panic
          println!(
              "  > {} at {}:{}", 
              symbol.name().unwrap(),
              symbol.filename().unwrap().to_string_lossy(),
              symbol.lineno().unwrap(),
          );
      }
  }
  }));

  panic!("hi");
  */

  nwg::init().expect("[main] failed to init nwg");

  //use Segoe UI with 16 size as default font
  let mut font = nwg::Font::default();
  nwg::Font::builder()
    .family("Segoe UI")
    .size(16)
    .build(&mut font).ok();
  nwg::Font::set_global_default(Some(font));

  //start the nwg app
  let svt_ui = ui::UI::build_ui(Default::default()).expect("[main] failed to build UI");

  //start svt
  let svt_app = svt::SVT{..Default::default()};

  //create reference to svt in ui
  svt_ui.init(svt_app);

  //initialize tooltips based off application control elements
  let mut tooltip = nwg::Tooltip::default();

  let _res_ = nwg::Tooltip::builder()
    .register(&svt_ui.inherited_text, "Paste timing point start/end pair(s) here. Copy/paste from timing panel. These timing points contain the start/end times, SVs, and volumes which are interpolated for the selected objects. (Example format: 111376,-76.92308,4,1,0,100,0,1)")
    .register(&svt_ui.lin_sv_check, "Change slider velocity linearly for selected objects (hits/barlines/inh. lines)")
    .register(&svt_ui.exp_sv_check, "Change slider velocity exponentially for selected objects (hits/barlines/inh. lines)")
    .register(&svt_ui.pol_sv_check, "Change slider velocity polynomially using exp. factor for selected objects (hits/barlines/inh. lines)")
    .register(&svt_ui.flat_sv_check, "Flat SV change for selected inh. lines")
    .register(&svt_ui.vol_check, "Change volume smoothly for selected objects (hits/barlines/inh. lines)")
    .register(&svt_ui.hit_check, "Change hitobjects (notes, spinners, sliders) between start/end points")
    .register(&svt_ui.barline_check, "Change barlines (big white lines) between start/end points")
    .register(&svt_ui.inh_check, "Change current inherited lines between start/end points")
    .register(&svt_ui.offset_label, "(integer) Place new timing points at offset (in ms) from map object (negative offset for before, positive for after)")
    .register(&svt_ui.buffer_label, "(integer) Include map objects (in ms) before and after the start/end timing points, useful if objects are not perfectly snapped")
    .register(&svt_ui.min_spacing_label, "(integer) Minimum spacing around tool-placed points (in ms) where other points must either follow social distancing or be removed")
    .register(&svt_ui.pol_exp_label, "(decimal) Exponent for polynomial SV. Recommended values are [0.5, 1) for slowdowns and (1.0, 2.0] for speedups. Applied following a (sv_diff) * (t / t_diff)^exp curve")
    .register(&svt_ui.flat_sv_label, "(decimal) Amount of SV change to apply to each inherited line")
    .register(&svt_ui.ign_bpm_check, "End timing point SV is normally relative to end timing point BPM, but if checked, can be made relative to start timing point BPM")
    .register(&svt_ui.open_button, "Select map to change")
    .register(&svt_ui.in_filename, "Map being edited")
    .register(&svt_ui.out_filename, "Output location")
    .register(&svt_ui.preview_check, "If enabled, creates a preview diff alongside your current diff which shows how the changes would potentially look without touching the original")
    .register(&svt_ui.apply_button, "Apply SV/vol changes")
    .register(&svt_ui.undo_button, "Undo most recent change")
    .build(&mut tooltip);
  tooltip.set_delay_time(Some(50));

  //start handling events
  nwg::dispatch_thread_events();
}