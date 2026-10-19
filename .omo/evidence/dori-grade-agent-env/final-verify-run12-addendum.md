# Run 12 addendum (follow-up fixes)

final-verify.json: fmt 0, clippy 0, build 0; 1146 tests passed; one new failure:
test_lane_status_transport::milestones_status_replacement_and_restart_use_owned_loopback - it expected a status edit on a
closed lane. That edit is exactly what crashed production (Discord 50083 on archived threads); posts.rs now skips closed lanes.
Test-only change: the closed step asserts no request; the target passes (1/1). Product source unchanged since the run.
Other failures equal the todo-2 baseline. Release binary: target-rel198 (cargo +1.98.1 build --release), built from the same product source.
