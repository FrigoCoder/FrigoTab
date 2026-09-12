namespace FrigoTab {

    public enum WindowMessages {

        KeyDown = 0x0100,
        KeyUp = 0x0101,
        SysKeyDown = 0x0104,
        SysKeyUp = 0x0105,

        ActivateApp = 0x001c,
        QueryEndSession = 0x0011,
        EndSessionNative = 0x0016,
        DisplayChange = 0x007e,
        DpiChanged = 0x02e0,
        DwmCompositionChanged = 0x031e,
        GetIcon = 0x007f,

        User = 0x4000,
        BeginSession = User + 1,
        EndSession = User + 2

    }

}
