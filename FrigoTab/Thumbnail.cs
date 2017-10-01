﻿using System;
using System.Drawing;

namespace FrigoTab {

    public class Thumbnail : IDisposable {

        private IntPtr _thumbnail;

        public Thumbnail (WindowHandle source, WindowHandle destination) {
            Dwm.DwmRegisterThumbnail(destination, source, out _thumbnail);
        }

        ~Thumbnail () {
            Dispose();
        }

        public void Dispose () {
            if( _thumbnail == IntPtr.Zero ) {
                return;
            }
            Dwm.DwmUnregisterThumbnail(_thumbnail);
            _thumbnail = IntPtr.Zero;
            GC.SuppressFinalize(this);
        }

        public Size GetSourceSize () {
            Size size;
            Dwm.DwmQueryThumbnailSourceSize(_thumbnail, out size);
            return size;
        }

        public void SetSourceRect (Rect sourceRect) {
            Dwm.ThumbnailProperties properties = new Dwm.ThumbnailProperties {
                Flags = Dwm.ThumbnailFlags.RectSource,
                Source = sourceRect
            };
            Dwm.DwmUpdateThumbnailProperties(_thumbnail, ref properties);
        }

        public void SetDestinationRect (Rect destinationRect) {
            Dwm.ThumbnailProperties properties = new Dwm.ThumbnailProperties {
                Flags = Dwm.ThumbnailFlags.RectDestination,
                Destination = destinationRect
            };
            Dwm.DwmUpdateThumbnailProperties(_thumbnail, ref properties);
        }

    }

}