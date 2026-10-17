use super::*;
use std::thread;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Message {
    text: String,
    state: Option<StateRef>,
}

fn link_name(test: &str) -> String {
    format!("test-{}-{test}", std::process::id())
}

fn link(test: &str) -> ((Sender, Receiver), (Sender, Receiver)) {
    let name = link_name(test);
    let listener = Listener::create(&name).unwrap();
    let player = thread::spawn(move || connect(&name).unwrap());
    let editor = listener.accept().unwrap();
    (editor, player.join().unwrap())
}

fn large_state(seed: u8) -> Vec<u8> {
    (0..1_300_001u32)
        .map(|index| (index as u8).wrapping_mul(31).wrapping_add(seed))
        .collect()
}

#[test]
fn messages_and_large_states_travel_both_ways() {
    let ((mut editor_out, mut editor_in), (mut player_out, mut player_in)) = link("roundtrip");

    // The player waits on its receiver in another thread, as a resident player does.
    let player = thread::spawn(move || {
        let request: Message = player_in.receive().unwrap().unwrap();
        let state = read_state(request.state.as_ref().unwrap()).unwrap();
        let reply_state = player_out.share(&large_state(7)).unwrap();
        player_out
            .send(&Message {
                text: format!("{} {}", request.text, state.len()),
                state: Some(reply_state.reference.clone()),
            })
            .unwrap();
        let done: Message = player_in.receive().unwrap().unwrap();
        assert_eq!(done.text, "done");
        assert_eq!(state, large_state(3));
    });

    let sent = editor_out.share(&large_state(3)).unwrap();
    editor_out
        .send(&Message {
            text: "open\npatch".into(),
            state: Some(sent.reference.clone()),
        })
        .unwrap();
    let reply: Message = editor_in.receive().unwrap().unwrap();
    assert_eq!(reply.text, "open\npatch 1300001");
    let received = read_state(reply.state.as_ref().unwrap()).unwrap();
    assert_eq!(received, large_state(7));
    drop(sent);
    editor_out
        .send(&Message {
            text: "done".into(),
            state: None,
        })
        .unwrap();
    player.join().unwrap();
}

#[test]
fn closing_the_editor_ends_the_player_receiver() {
    let ((editor_out, editor_in), (_player_out, mut player_in)) = link("disconnect");
    let player = thread::spawn(move || player_in.receive::<Message>().unwrap());
    drop((editor_out, editor_in));
    assert_eq!(player.join().unwrap(), None);
}

#[test]
fn a_state_is_gone_once_both_sides_have_closed_it() {
    let ((mut editor_out, _editor_in), _player) = link("state-lifetime");
    let shared = editor_out.share(b"state").unwrap();
    let reference = shared.reference.clone();
    assert_eq!(read_state(&reference).unwrap(), b"state");
    drop(shared);
    assert!(read_state(&reference).is_err());
    let empty = editor_out.share(&[]).unwrap();
    assert_eq!(read_state(&empty.reference).unwrap(), Vec::<u8>::new());
    assert_ne!(empty.reference.name, reference.name);
}

#[test]
fn a_second_listener_with_the_same_name_is_refused() {
    let name = link_name("duplicate");
    let _first = Listener::create(&name).unwrap();
    assert!(Listener::create(&name).is_err());
}
