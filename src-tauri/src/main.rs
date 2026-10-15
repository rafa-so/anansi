// Sem janela de console no build de release do Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    anansi_lib::run()
}
