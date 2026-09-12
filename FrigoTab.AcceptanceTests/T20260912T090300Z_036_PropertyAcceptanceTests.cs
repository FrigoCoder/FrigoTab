using System.Collections.Generic;
using FrigoTab;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("CurrentFeature")]
    [TestCategory("Regression")]
    public sealed class T20260912T090300Z_036_PropertyAcceptanceTests {

        [TestMethod]
        public void EqualAssignmentIsSilentAndChangedAssignmentNotifiesOnce () {
            var property = new Property<string>();
            var changes = new List<PropertyChange>();
            property.Value = "before";
            property.Changed += (oldValue, newValue) => changes.Add(new PropertyChange(oldValue, newValue));

            property.Value = "before";
            property.Value = "after";

            Assert.AreEqual(1, changes.Count);
            Assert.AreEqual("before", changes[0].OldValue);
            Assert.AreEqual("after", changes[0].NewValue);
        }

        private sealed class PropertyChange {

            public PropertyChange (string oldValue, string newValue) {
                OldValue = oldValue;
                NewValue = newValue;
            }

            public string OldValue { get; }
            public string NewValue { get; }

        }

    }

}
