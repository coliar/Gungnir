mod poem;
mod uname;
mod meminfo;

pub(super) fn add_cmds() {
    poem::add_cmd();
    uname::add_cmd();
    meminfo::add_cmd();
}