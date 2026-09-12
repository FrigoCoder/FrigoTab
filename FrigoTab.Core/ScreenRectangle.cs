using System;

namespace FrigoTab.Core {

    /// <summary>
    /// A screen-coordinate rectangle independent of System.Drawing and WinForms.
    /// Coordinates may be negative for monitors positioned left or above the
    /// primary display.
    /// </summary>
    public struct ScreenRectangle : IEquatable<ScreenRectangle> {

        public ScreenRectangle (int x, int y, int width, int height) {
            X = x;
            Y = y;
            Width = width;
            Height = height;
        }

        public int X { get; }
        public int Y { get; }
        public int Width { get; }
        public int Height { get; }
        public int Right => X + Width;
        public int Bottom => Y + Height;
        public bool IsEmpty => Width <= 0 || Height <= 0;

        public bool Contains (ScreenPoint point) =>
            !IsEmpty && point.X >= X && point.X < Right && point.Y >= Y && point.Y < Bottom;

        public bool Equals (ScreenRectangle other) =>
            X == other.X && Y == other.Y && Width == other.Width && Height == other.Height;

        public override bool Equals (object obj) => obj is ScreenRectangle && Equals((ScreenRectangle) obj);

        public override int GetHashCode () {
            unchecked {
                int hash = X;
                hash = (hash * 397) ^ Y;
                hash = (hash * 397) ^ Width;
                return (hash * 397) ^ Height;
            }
        }

        public override string ToString () => "(" + X + ", " + Y + ", " + Width + ", " + Height + ")";

        public static bool operator == (ScreenRectangle left, ScreenRectangle right) => left.Equals(right);
        public static bool operator != (ScreenRectangle left, ScreenRectangle right) => !left.Equals(right);

    }

}
