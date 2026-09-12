using System;

namespace FrigoTab.Core {

    /// <summary>
    /// A screen-coordinate point.  Keeping this type independent of
    /// System.Drawing makes the acceptance boundary usable without WinForms.
    /// </summary>
    public struct ScreenPoint : IEquatable<ScreenPoint> {

        public ScreenPoint (int x, int y) {
            X = x;
            Y = y;
        }

        public int X { get; }
        public int Y { get; }

        public bool Equals (ScreenPoint other) => X == other.X && Y == other.Y;

        public override bool Equals (object obj) => obj is ScreenPoint && Equals((ScreenPoint) obj);

        public override int GetHashCode () {
            unchecked {
                return (X * 397) ^ Y;
            }
        }

        public override string ToString () => "(" + X + ", " + Y + ")";

        public static bool operator == (ScreenPoint left, ScreenPoint right) => left.Equals(right);
        public static bool operator != (ScreenPoint left, ScreenPoint right) => !left.Equals(right);

    }

}
