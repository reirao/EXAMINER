mod components;

use std::sync::{
	LazyLock,
	atomic::{AtomicBool, AtomicU64, Ordering},
};

use hudhook::*;
use tracing::error;
use windows::Win32::UI::{
	Input::KeyboardAndMouse,
	WindowsAndMessaging::{
		CURSOR_SHOWING, CURSORINFO, GetCursorInfo, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
		WM_LBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
	},
};

use crate::plugins::manager::PluginManager;

const EMTK_FRAMEWORK_VERSION: &str = env!("CARGO_PKG_VERSION");
const EMTK_FRAMEWORK_REPO: &str = env!("CARGO_PKG_REPOSITORY");
const EMTK_FRAMEWORK_DOCS: &str = env!("CARGO_PKG_HOMEPAGE");
const EMTK_FRAMEWORK_LICENSE: &str = env!("CARGO_PKG_LICENSE");

static EMTK_FRAMEWORK_AUTHORS: LazyLock<String> = LazyLock::new(|| {
	env!("CARGO_PKG_AUTHORS")
		.split(":")
		.map(|s| {
			if s.contains(" <") {
				s.split(" <").collect::<Vec<&str>>()[0]
			} else {
				s
			}
		})
		.collect::<Vec<&str>>()
		.join(", ")
});

pub(crate) fn inject_gui() {
	use hudhook::hooks::opengl3::ImguiOpenGl3Hooks;
	std::thread::spawn(move || {
		let result = Hudhook::builder()
			.with::<ImguiOpenGl3Hooks>(RenderLoop::default())
			.build()
			.apply();

		if let Err(e) = result {
			error!("Failed to apply HUD hook: {:?}", e);
		}
	});
}

pub trait Widget {
	fn render(&mut self, ui: &imgui::Ui);
	fn initialize(&mut self, _ctx: &mut imgui::Context, _render_context: &mut dyn RenderContext) {}
}

static VISIBILITY_TOGGLE_KEY_DOWN: AtomicBool = AtomicBool::new(false);
static VISIBILITY_TOGGLED: AtomicBool = AtomicBool::new(false);
static EXPERIMENT_TOGGLE_KEY_DOWN: AtomicBool = AtomicBool::new(false);
static EXPERIMENT_TOGGLED: AtomicBool = AtomicBool::new(false);
static LEFT_MOUSE_DOWN: AtomicBool = AtomicBool::new(false);
static RIGHT_MOUSE_DOWN: AtomicBool = AtomicBool::new(false);
static CTRL_DOWN: AtomicBool = AtomicBool::new(false);
static SHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static ALT_DOWN: AtomicBool = AtomicBool::new(false);
static INPUT_EVENTS: AtomicU64 = AtomicU64::new(0);

#[derive(Default)]
pub struct RenderLoop {
	components: Vec<Box<dyn Widget + Send + Sync>>,
	menu: components::Menu,
	state: AppState,
	logo_texture: Option<(imgui::TextureId, u32, u32)>,
}

pub struct AppState {
	experiment_enabled: bool,
	visible: bool,
	show_cursor: bool,
	show_demo_window: bool,
	show_about_window: bool,
}

impl Default for AppState {
	fn default() -> Self {
		Self {
			experiment_enabled: false,
			#[cfg(debug_assertions)]
			visible: true,
			#[cfg(not(debug_assertions))]
			visible: false,
			show_cursor: false,
			show_demo_window: false,
			show_about_window: false,
		}
	}
}

impl ImguiRenderLoop for RenderLoop {
	fn initialize(&mut self, ctx: &mut imgui::Context, render_context: &mut dyn RenderContext) {
		let io = ctx.io_mut();
		io.config_windows_move_from_title_bar_only = true;
		io.config_flags |= imgui::ConfigFlags::DOCKING_ENABLE;
		io.config_docking_with_shift = true;

		// Stop events being queued up when the window is not visible.
		io.config_input_trickle_event_queue = false;

		let style = ctx.style_mut();
		style.window_title_align = [0.5, 0.5];

		// TODO: Can the font rendering be improved? It looks blurry.
		// Maybe another font, or something else? FreeType is an option but could be annoying to implement.
		let fonts = ctx.fonts();
		fonts.add_font(&[imgui::FontSource::TtfData {
			data: include_bytes!("../../../../assets/fonts/Lora-Regular.ttf"),
			size_pixels: 20.0,
			config: Some(imgui::FontConfig {
				size_pixels: 20.0,
				oversample_h: 3,
				oversample_v: 3,
				..Default::default()
			}),
		}]);

		let plugin_ids = PluginManager::get_ids();

		let plugins: Vec<components::Plugin> = plugin_ids
			.into_iter()
			.map(|id| {
				let config = PluginManager::get_info_for(&id).unwrap();
				components::Plugin::from(config)
			})
			.collect();

		let mut plugins = components::Plugins::from(plugins);
		plugins.initialize(ctx, render_context);
		self.components.push(Box::new(plugins));

		self.menu = components::Menu::new(vec!["Plugins".to_string()]);

		let image = image::load(
			std::io::Cursor::new(include_bytes!(
				"../../../../assets/images/corro-211x172.png"
			)),
			image::ImageFormat::Png,
		)
		.unwrap()
		.to_rgba8();
		let dimensions = image.dimensions();

		self.logo_texture = match render_context.load_texture(
			image.into_raw().as_slice(),
			dimensions.0,
			dimensions.1,
		) {
			Ok(texture) => Some((texture, dimensions.0, dimensions.1)),
			Err(e) => {
				error!("Failed to load logo texture: {:?}", e);
				None
			}
		};
	}

	fn on_wnd_proc(
		&self,
		_hwnd: windows::Win32::Foundation::HWND,
		umsg: u32,
		_wparam: windows::Win32::Foundation::WPARAM,
		_lparam: windows::Win32::Foundation::LPARAM,
	) {
		match umsg {
			WM_LBUTTONDOWN => LEFT_MOUSE_DOWN.store(true, Ordering::Relaxed),
			WM_LBUTTONUP => LEFT_MOUSE_DOWN.store(false, Ordering::Relaxed),
			WM_RBUTTONDOWN => RIGHT_MOUSE_DOWN.store(true, Ordering::Relaxed),
			WM_RBUTTONUP => RIGHT_MOUSE_DOWN.store(false, Ordering::Relaxed),
			WM_KEYDOWN | WM_KEYUP => {
				let down = umsg == WM_KEYDOWN;
				let key = _wparam.0 as u16;
				if key == KeyboardAndMouse::VK_F2.0 {
					VISIBILITY_TOGGLE_KEY_DOWN.store(down, Ordering::Relaxed);
				} else if key == KeyboardAndMouse::VK_F6.0 {
					EXPERIMENT_TOGGLE_KEY_DOWN.store(down, Ordering::Relaxed);
				} else if key == KeyboardAndMouse::VK_CONTROL.0 {
					CTRL_DOWN.store(down, Ordering::Relaxed);
				} else if key == KeyboardAndMouse::VK_SHIFT.0 {
					SHIFT_DOWN.store(down, Ordering::Relaxed);
				} else if key == KeyboardAndMouse::VK_MENU.0 {
					ALT_DOWN.store(down, Ordering::Relaxed);
				}
			}
			_ => return,
		}
		INPUT_EVENTS.fetch_add(1, Ordering::Relaxed);
	}

	fn before_render<'a>(
		&'a mut self,
		ctx: &mut imgui::Context,
		_render_context: &'a mut dyn RenderContext,
	) {
		let io = ctx.io_mut();
		let visibility_down = VISIBILITY_TOGGLE_KEY_DOWN.load(Ordering::Relaxed);
		if visibility_down && !VISIBILITY_TOGGLED.swap(visibility_down, Ordering::Relaxed) {
			self.state.visible = !self.state.visible;
		} else if !visibility_down {
			VISIBILITY_TOGGLED.store(false, Ordering::Relaxed);
		}

		let experiment_down = EXPERIMENT_TOGGLE_KEY_DOWN.load(Ordering::Relaxed);
		if experiment_down && !EXPERIMENT_TOGGLED.swap(experiment_down, Ordering::Relaxed) {
			self.state.experiment_enabled = !self.state.experiment_enabled;
		} else if !experiment_down {
			EXPERIMENT_TOGGLED.store(false, Ordering::Relaxed);
		}

		// If the window is not visible, set scale to 0 to disable rendering.
		io.display_framebuffer_scale = if self.state.visible {
			[1., 1.]
		} else {
			[0., 0.]
		};

		// Clear events if the window is not visible.
		if !self.state.visible {
			io.mouse_down = [false; 5];
			io.keys_down = [false; 652];
		}

		unsafe {
			let mut cursor_info: CURSORINFO = std::mem::zeroed();
			cursor_info.cbSize = std::mem::size_of::<CURSORINFO>() as _;
			if GetCursorInfo(&raw mut cursor_info).ok().is_some() {
				let cursor_is_visible = cursor_info.flags.0 & CURSOR_SHOWING.0 == CURSOR_SHOWING.0;

				io.mouse_draw_cursor = !cursor_is_visible && self.state.show_cursor;
			}
		}
	}

	fn render(&mut self, ui: &mut imgui::Ui) {
		if !self.state.visible {
			ui.window("Hidden Window")
				.position([0., 0.], imgui::Condition::FirstUseEver)
				.size([10., 10.], imgui::Condition::FirstUseEver)
				.draw_background(false)
				.build(|| {});
			return;
		}

		// If we have multiple windows, if any of them are hovered, show the cursor.
		let mut is_window_hovered = false;

		if self.state.show_demo_window {
			ui.show_demo_window(&mut self.state.show_demo_window);
		}

		if self.state.show_about_window {
			ui.window("About")
				.opened(&mut self.state.show_about_window)
				.collapsible(false)
				.size([450., 260.], imgui::Condition::FirstUseEver)
				.build(|| {
					if let Some(_table) = ui.begin_table_with_flags(
						"##AboutTable",
						2,
						imgui::TableFlags::SIZING_FIXED_FIT,
					) {
						ui.table_next_column();

						ui.text("EXAMINER Framework");
						ui.text(format!("Version: {}", EMTK_FRAMEWORK_VERSION));
						ui.text(format!("Authors: {}", &*EMTK_FRAMEWORK_AUTHORS));
						ui.text(format!("License: {}", EMTK_FRAMEWORK_LICENSE));

						if ui.button("View Source") {
							open::that(EMTK_FRAMEWORK_REPO).unwrap();
						}

						if ui.button("View Docs") {
							open::that(EMTK_FRAMEWORK_DOCS).unwrap();
						}

						ui.table_next_column();

						if let Some((logo, w, h)) = self.logo_texture {
							imgui::Image::new(logo, [w as f32, h as f32]).build(ui);
						}
					}

					ui.separator();
					ui.text_wrapped("You can press F2 to show/hide the toolkit overlay.");
				});
		}

		ui.window("EXAMINER Diagnostics")
			.position([20., 20.], imgui::Condition::FirstUseEver)
			.size([360., 230.], imgui::Condition::FirstUseEver)
			.build(|| {
				let status = if self.state.experiment_enabled {
					"ARMED"
				} else {
					"SAFE / OBSERVE ONLY"
				};
				ui.text(format!("Experiment state: {status}"));
				ui.text("F2: toggle overlay | F6: arm experiments");
				ui.separator();
				ui.text(format!(
					"Left mouse:  {}",
					if LEFT_MOUSE_DOWN.load(Ordering::Relaxed) { "DOWN" } else { "up" }
				));
				ui.text(format!(
					"Right mouse: {}",
					if RIGHT_MOUSE_DOWN.load(Ordering::Relaxed) { "DOWN" } else { "up" }
				));
				ui.text(format!(
					"Ctrl: {}   Shift: {}   Alt: {}",
					CTRL_DOWN.load(Ordering::Relaxed),
					SHIFT_DOWN.load(Ordering::Relaxed),
					ALT_DOWN.load(Ordering::Relaxed),
				));
				ui.text(format!(
					"Captured input events: {}",
					INPUT_EVENTS.load(Ordering::Relaxed)
				));
				ui.separator();
				ui.text_wrapped(
					"Telemetry is active. No gameplay or physics values are modified yet.",
				);
			});

		ui.window("EXAMINER Framework")
			.position([0., 0.], imgui::Condition::FirstUseEver)
			.size([650., 400.], imgui::Condition::FirstUseEver)
			.menu_bar(true)
			.build(|| {
				ui.dockspace_over_main_viewport();

				ui.menu_bar(|| {
					ui.menu("Menu", || {
						ui.menu_item("Open Mods Folder");
						if ui.is_item_clicked_with_button(imgui::MouseButton::Left) {
							std::fs::create_dir_all("mods").unwrap();
							std::process::Command::new("explorer")
								.arg("mods")
								.spawn()
								.unwrap();
						}
					});

					#[cfg(debug_assertions)]
					ui.menu("Debug", || {
						ui.menu("Dear ImGui", || {
							ui.menu_item("Show Demo Window");
							if ui.is_item_clicked_with_button(imgui::MouseButton::Left) {
								self.state.show_demo_window = !self.state.show_demo_window;
							}
						});
					});

					ui.menu("Help", || {
						ui.menu_item("About");
						if ui.is_item_clicked_with_button(imgui::MouseButton::Left) {
							self.state.show_about_window = true;
						}
					})
				});

				self.menu.render(ui);

				if self.menu.selected.as_str() == "Plugins" {
					for component in &mut self.components {
						component.render(ui);
					}
				}

				if ui.is_window_hovered() {
					is_window_hovered = true;
				}
			});

		self.state.show_cursor = ui.is_any_item_hovered()
			|| ui.is_any_item_active()
			|| ui.is_mouse_dragging(imgui::MouseButton::Left)
			|| is_window_hovered;
	}

	fn message_filter(&self, io: &imgui::Io) -> MessageFilter {
		if self.state.visible
			&& (io.want_capture_mouse || io.want_capture_keyboard || io.want_text_input)
		{
			return MessageFilter::InputAll;
		}

		MessageFilter::empty()
	}
}
