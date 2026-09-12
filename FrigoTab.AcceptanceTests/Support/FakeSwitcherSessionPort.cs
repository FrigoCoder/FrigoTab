using System;
using System.Collections.Generic;
using FrigoTab.Core;

namespace FrigoTab.AcceptanceTests.Support {

    /// <summary>
    /// In-memory desktop/session boundary used by the executable acceptance
    /// scenarios.  It records the observable calls made by the application
    /// state machine without creating HWNDs, hooks, or DWM resources.
    /// </summary>
    public sealed class FakeSwitcherSessionPort : ISwitcherSessionPort {

        private int? currentSelection;

        public FakeSwitcherSessionPort (int candidateCount = 3) {
            CandidateCount = candidateCount;
            OpenResult = true;
            ActivationResult = true;
        }

        public int CandidateCount { get; set; }
        public bool OpenResult { get; set; }
        public bool ActivationResult { get; set; }
        public bool ThrowOnOpen { get; set; }
        public bool ThrowOnSelect { get; set; }
        public bool ThrowOnClearSelection { get; set; }
        public bool ThrowOnHitTest { get; set; }
        public bool ThrowOnActivation { get; set; }
        public bool ThrowOnRelayout { get; set; }
        public bool ThrowOnClose { get; set; }
        public int? HitTestResult { get; set; }

        public bool IsOpen { get; private set; }
        public int OpenCalls { get; private set; }
        public int CloseCalls { get; private set; }
        public int SelectCalls { get; private set; }
        public int ClearSelectionCalls { get; private set; }
        public int HitTestCalls { get; private set; }
        public int ActivateCalls { get; private set; }
        public int RelayoutCalls { get; private set; }
        public int? CurrentSelection => currentSelection;

        public IList<int> SelectedIndices { get; } = new List<int>();
        public IList<int?> ActivatedSelections { get; } = new List<int?>();
        public IList<string> Events { get; } = new List<string>();

        public bool TryOpen (out int candidateCount) {
            OpenCalls++;
            Events.Add("open");
            if( ThrowOnOpen ) {
                throw new InvalidOperationException("The fake session was configured to fail while opening.");
            }

            candidateCount = CandidateCount;
            IsOpen = OpenResult && candidateCount > 0;
            return OpenResult;
        }

        public void Select (int index) {
            SelectCalls++;
            Events.Add("select:" + index);
            if( ThrowOnSelect ) {
                throw new InvalidOperationException("The fake session was configured to fail while selecting.");
            }

            currentSelection = index;
            SelectedIndices.Add(index);
        }

        public void ClearSelection () {
            ClearSelectionCalls++;
            Events.Add("clear-selection");
            if( ThrowOnClearSelection ) {
                throw new InvalidOperationException("The fake session was configured to fail while clearing selection.");
            }

            currentSelection = null;
        }

        public int? HitTest (ScreenPoint point) {
            HitTestCalls++;
            Events.Add("hit-test:" + point);
            if( ThrowOnHitTest ) {
                throw new InvalidOperationException("The fake session was configured to fail during hit testing.");
            }

            return HitTestResult;
        }

        public bool TryActivateSelected () {
            ActivateCalls++;
            Events.Add("activate");
            ActivatedSelections.Add(currentSelection);
            if( ThrowOnActivation ) {
                throw new InvalidOperationException("The fake session was configured to fail during activation.");
            }

            return ActivationResult;
        }

        public void Close () {
            CloseCalls++;
            Events.Add("close");
            IsOpen = false;
            currentSelection = null;
            if( ThrowOnClose ) {
                throw new InvalidOperationException("The fake session was configured to fail while closing.");
            }
        }

        public void Relayout () {
            RelayoutCalls++;
            Events.Add("relayout");
            if( ThrowOnRelayout ) {
                throw new InvalidOperationException("The fake session was configured to fail during relayout.");
            }
        }

    }

}
