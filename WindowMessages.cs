namespace FrigoTab {

    public enum WindowMessages {

        KeyDown = 0x0100,
        KeyUp = 0x0101,
        SysKeyDown = 0x0104,
        SysKeyUp = 0x0105,

        ActivateApp = 0x001c,
        DisplayChange = 0x007e,
        GetIcon = 0x007f,

        MouseMove = 0x200,
        LeftDown = 0x201,
        LeftUp = 0x202,
        RightDown = 0x204,
        RightUp = 0x205,
        MouseWheel = 0x20a,
        MouseHWheel = 0x20e,

        User = 0x4000,
        BeginSession = User + 1,
        EndSession = User + 2,
        KeyPressed = User + 3,
        MouseMoved = User + 4,
        MouseClicked = User + 5

    }

}
