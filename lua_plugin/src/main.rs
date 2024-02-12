use nih_plug::nih_export_standalone;

use lua_plugin::LuaPlugin;

fn main() {
    nih_export_standalone::<LuaPlugin>();
}
