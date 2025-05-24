class State:
    def __init__(self):
        self.prev = []


def process(state, buf, events, transport):
    if state is None:
        state = State()
    for c in range(len(buf)):
        if len(state.prev) <= c:
            state.prev.append([])
        for i in range(len(buf[c])):
            if len(state.prev[c]) <= i:
                state.prev[c][i] = 0.0
            buf[c][i] = buf[c][i] * 0.5 + state.prev[c][i] * 0.5
            state.prev[c][i] = buf[c][i]
    return (state, buf, events)
