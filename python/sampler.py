

def process(state, buf, events):
    class State():
        def __init__(self):
            self.data = []
            self.write = 0
            self.read = 0
            self.playing = False
            self.playing_note = 0
            self.recording = False

    nc = len(buf)
    ns = len(buf[0])
    if state is None:
        state = State()
        state.data = list([] for _ in range(nc))
    event_idx = 0
    for sample_idx in range(ns):
        while event_idx < len(events) and events[event_idx].timing == sample_idx:
            e = events[event_idx]
            if isinstance(e, host.NoteOn) and e.note == 0:
                state.recording = True
                state.write = 0
                host.print("START RECORDING")
            elif isinstance(e, host.NoteOff) and e.note == 0:
                state.recording = False
                for c in range(nc):
                    state.data[c] = state.data[c][:state.write]
                state.write = 0
                host.print("STOP RECORDING", state.write, len(state.data[0]))
            elif isinstance(e, host.NoteOn) and (e.note >= 12 and e.note < 12 + 16):
                state.read = int(len(state.data[0]) * (float(e.note - 12) / 16.0))
                state.playing = True
                state.playing_note = e.note
                host.print("PLAY FROM", e.note, state.read)
            elif isinstance(e, host.NoteOff) and e.note == state.playing_note:
                state.playing = False
                state.playing_note = 0
                host.print("STOP PLAY", e.note)
            else:
                host.print(e.note)
            event_idx += 1
        if state.recording:
            if state.write == len(state.data[0]):
                for c in range(nc):
                    state.data[c].append(buf[c][sample_idx])
            else:
                for c in range(nc):
                    state.data[c][state.write] = buf[c][sample_idx]
            state.write = state.write + 1
        if state.playing:
            for c in range(nc):
                buf[c][sample_idx] = state.data[c][state.read]
            state.read = state.read + 1
            if state.read == len(state.data[0]):
                state.read = 0



#    for e in events:
#        t.append(e.timing)
#        if isinstance(e, host.NoteOn) and e.note == 0:
#            state.recording = True
#            host.print("YES")
#        elif isinstance(e, host.NoteOff) and e.note == 0:
#            state.recording = False
#            host.print("NO", state.write, len(state.data))


#        if isinstance(e, host.NoteOn) and e.note == 0:
#            state.recording = True
#            host.print("YES")
#        elif isinstance(e, host.NoteOff) and e.note == 0:
#            state["recording"] = False
#            host.print("NO", state["write"], len(state["data"]))
#            state["write"] = 0
#
#    if state["recording"]:
#        w = state["write"]
#        if w == len(state["data"]):
#            state["data"].append(buf)
#        else:
#            state["data"][w] = buf
#        state["write"] = (w + 1) % 100
    return (state, buf, events)
