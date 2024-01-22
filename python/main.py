# import rtmidi
# m = rtmidi.RtMidiOut()
# for i in range(m.getPortCount()):
#     print(i, m.getPortName(i))
#print(midiout.get_ports())
#port = mido.open_output('VirtualMidi Port1')
#port.send(msg)
import attrs
import math
import typing as t


@attrs.define
class Interval():
    start: int
    end: int

@attrs.define
class Intervals():
    intervals: t.List[Interval]
    def duration(self) -> int:
        return sum([abs(interval.end - interval.start) for interval in self.intervals])
    def project(self, x: int) -> t.List[Interval]:
        offset = 0
        for interval in self.intervals:
            s = interval.start
            e = interval.end
            d = abs(e - s)
            xd = x - offset
            if xd >= 0 and xd < d:
                if s < e:
                    return s + xd
                else:
                    return s - xd - 1
            offset += d
        raise Exception(f"x out of range: {x} not in [0, {self.duration()}]")

def project_list(view: Intervals, data: t.List[int]) -> t.List[int]:
    return list(data[view.project(x)] for x in range(0, view.duration()))

def calc_loop(start: int, length: int, data_len: int) -> Intervals:
    out = []
    end = (start + length) % data_len
    if start < end:
        out.append(Interval(start, end))
        out.append(Interval(end, start))
    else:
        out.append(Interval(start, data_len))
        if end > 0:
            out.append(Interval(0, end))
            out.append(Interval(end, 0))
        out.append(Interval(data_len, start))


    view = Intervals(out)
    return view

def main():
    # view = Intervals([Interval(0, 10), Interval(30, 20)])
    data = list(range(0, 30))
    view = calc_loop(20, 15, len(data))
    for x in range(0, view.duration()):
        print(f"{x}: {view.project(x)}")

if __name__ == '__main__':
    main()
