mod poem;
mod uname;

pub(super) fn add_cmds() {
    poem::add_cmd();
    uname::add_cmd();
}