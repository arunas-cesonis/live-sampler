

def process(state, buf, events):
    if state is None:
        state = {"data":[], "write":0, "recording":False}
    for e in events:
        if isinstance(e, host.NoteOn) and e.note == 0:
            state["recording"] = True
            host.print("YES")
        elif isinstance(e, host.NoteOff) and e.note == 0:
            state["recording"] = False
            host.print("NO", state["write"], len(state["data"]))
            state["write"] = 0

    if state["recording"]:
        w = state["write"]
        if w == len(state["data"]):
            state["data"].append(buf)
        else:
            state["data"][w] = buf
        state["write"] = (w + 1) % 100

    return (state, buf, events)