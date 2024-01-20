# import rtmidi
# m = rtmidi.RtMidiOut()
# for i in range(m.getPortCount()):
#     print(i, m.getPortName(i))
#print(midiout.get_ports())
#port = mido.open_output('VirtualMidi Port1')
#port.send(msg)

def main():
    data = list(range(1, 11))
    length = len(data)
    length_parcent = 0.5
    start_percent = 0.8
    loop_length = length * length_parcent
    speed = 1.0
    read = start_percent * length
    ###
    print(read)

if __name__ == '__main__':
    a = 75
    b = 32
    x *=27
