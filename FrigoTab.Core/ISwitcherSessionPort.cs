namespace FrigoTab.Core {

    /// <summary>
    /// The boundary between the deterministic switcher state machine and the
    /// Win32/WinForms implementation.  Implementations own native windows,
    /// thumbnails, drawing resources, and foreground-window calls.
    /// </summary>
    public interface ISwitcherSessionPort {

        /// <summary>
        /// Attempts to construct and show a session.  The port must return the
        /// number of selectable candidates it made available.  A false result
        /// or a zero candidate count means that no session is usable.
        /// </summary>
        bool TryOpen (out int candidateCount);

        /// <summary>
        /// Selects a candidate by its zero-based index.
        /// </summary>
        void Select (int index);

        /// <summary>
        /// Clears the current selection when the pointer is outside every
        /// candidate.  This preserves the prototype's observable behavior.
        /// </summary>
        void ClearSelection ();

        /// <summary>
        /// Returns the candidate under a screen point, or null when the point
        /// is not over a selectable candidate.
        /// </summary>
        int? HitTest (ScreenPoint point);

        /// <summary>
        /// Attempts to activate the currently selected candidate.  A false
        /// result means the session must remain visible so the user can retry
        /// or cancel it.
        /// </summary>
        bool TryActivateSelected ();

        /// <summary>
        /// Hides the session and releases all resources.  This must be safe to
        /// call after a partial or failed open.
        /// </summary>
        void Close ();

        /// <summary>
        /// Recalculates the session for the current display topology and DPI.
        /// </summary>
        void Relayout ();

    }

}
