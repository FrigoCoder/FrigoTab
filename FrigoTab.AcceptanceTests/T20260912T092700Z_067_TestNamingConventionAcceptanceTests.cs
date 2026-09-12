using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text.RegularExpressions;
using Microsoft.VisualStudio.TestTools.UnitTesting;

namespace FrigoTab.AcceptanceTests {

    [TestClass]
    [TestCategory("Acceptance")]
    [TestCategory("ContractProbe")]
    public sealed class T20260912T092700Z_067_TestNamingConventionAcceptanceTests {

        private static readonly Regex RequiredClassName = new Regex(
            @"^(?<family>T\d{8}T\d{6}Z_\d{3})_(?<description>[A-Z][A-Za-z0-9]*)$",
            RegexOptions.CultureInvariant);

        private static readonly Regex OrdinaryMethodName = new Regex(
            @"^[A-Z][A-Za-z0-9]*$",
            RegexOptions.CultureInvariant);

        private static readonly Regex TestClassDeclaration = new Regex(
            @"\[TestClass(?:Attribute)?(?:\s*\([^]]*\))?\](?:\s*\[[^]]+\]\s*)*(?:(?:public|internal|private|protected|static|sealed|abstract|partial)\s+)*class\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)",
            RegexOptions.CultureInvariant);

        [TestMethod]
        public void TimestampedTestClassesUseMatchingSourceFilesAndOrdinaryMethods () {
            Type[] testClasses = Assembly.GetExecutingAssembly()
                .GetTypes()
                .Where(type => type.GetCustomAttribute<TestClassAttribute>() != null)
                .ToArray();

            string[] invalidClassNames = testClasses
                .Select(type => type.Name)
                .Where(name => !RequiredClassName.IsMatch(name))
                .OrderBy(name => name, StringComparer.Ordinal)
                .ToArray();
            Assert.AreEqual(
                0,
                invalidClassNames.Length,
                "Every test class needs an immutable UTC family prefix. Invalid: " +
                    String.Join(", ", invalidClassNames));

            string[] duplicateFamilyIds = testClasses
                .Select(type => RequiredClassName.Match(type.Name))
                .Where(match => match.Success)
                .GroupBy(match => match.Groups["family"].Value, StringComparer.Ordinal)
                .Where(group => group.Count() > 1)
                .Select(group => group.Key)
                .OrderBy(name => name, StringComparer.Ordinal)
                .ToArray();
            Assert.AreEqual(
                0,
                duplicateFamilyIds.Length,
                "Test class family identifiers must be unique. Duplicates: " +
                    String.Join(", ", duplicateFamilyIds));

            MethodInfo[] tests = testClasses
                .SelectMany(type => type.GetMethods(BindingFlags.Public | BindingFlags.Instance))
                .Where(method => method.GetCustomAttribute<TestMethodAttribute>() != null)
                .ToArray();
            string[] invalidMethodNames = tests
                .Select(method => method.Name)
                .Where(name => !OrdinaryMethodName.IsMatch(name) || RequiredClassName.IsMatch(name))
                .OrderBy(name => name, StringComparer.Ordinal)
                .ToArray();
            Assert.AreEqual(
                0,
                invalidMethodNames.Length,
                "Test methods must use ordinary PascalCase names without family prefixes. Invalid: " +
                    String.Join(", ", invalidMethodNames));

            string testSourceDirectory = Path.Combine(FindRepositoryRoot(), "FrigoTab.AcceptanceTests");
            string[] sourceFiles = Directory.GetFiles(
                    testSourceDirectory,
                    "*.cs",
                    SearchOption.AllDirectories)
                .Where(path => File.ReadAllText(path).Contains("[TestClass]", StringComparison.Ordinal))
                .ToArray();
            HashSet<string> classNames = new HashSet<string>(
                testClasses.Select(type => type.Name),
                StringComparer.Ordinal);
            string[] invalidSourceFileNames = sourceFiles
                .Select(path => {
                    string source = File.ReadAllText(path);
                    Match declaration = TestClassDeclaration.Match(source);
                    string stem = Path.GetFileNameWithoutExtension(path);
                    string declaredClass = declaration.Success
                        ? declaration.Groups["name"].Value
                        : "<no class declaration>";
                    return new {
                        Stem = stem,
                        DeclaredClass = declaredClass,
                        IsValid = declaration.Success &&
                            RequiredClassName.IsMatch(stem) &&
                            classNames.Contains(declaredClass) &&
                            String.Equals(stem, declaredClass, StringComparison.Ordinal)
                    };
                })
                .Where(item => !item.IsValid)
                .Select(item => item.Stem + " => " + item.DeclaredClass)
                .OrderBy(name => name, StringComparer.Ordinal)
                .ToArray();
            Assert.AreEqual(
                0,
                invalidSourceFileNames.Length,
                "Every [TestClass] source file must have the exact timestamped class name as its stem. Invalid: " +
                    String.Join(", ", invalidSourceFileNames));

            string[] missingSourceFiles = testClasses
                .Select(type => Path.Combine(testSourceDirectory, type.Name + ".cs"))
                .Where(path => !File.Exists(path))
                .Select(path => Path.GetFileName(path))
                .OrderBy(name => name, StringComparer.Ordinal)
                .ToArray();
            Assert.AreEqual(
                0,
                missingSourceFiles.Length,
                "Every test class must have a source file with the same timestamped name. Missing: " +
                    String.Join(", ", missingSourceFiles));
        }

        private static string FindRepositoryRoot () {
            DirectoryInfo directory = new DirectoryInfo(AppContext.BaseDirectory);
            while( directory != null ) {
                if( File.Exists(Path.Combine(directory.FullName, "FrigoTab.sln")) ) {
                    return directory.FullName;
                }
                directory = directory.Parent;
            }
            throw new DirectoryNotFoundException(
                "Could not locate the FrigoTab repository root for the test naming contract.");
        }

    }

}
