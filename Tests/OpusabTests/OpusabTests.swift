import Foundation
import XCTest
import Files
import OpusabCore

class OpusabTests: XCTestCase {
    var testFolder: Folder! = nil
    
    override func setUp() {
        // Setup a temp test folder that can be used as a sandbox
        let fileSystem = FileSystem()
        let tempFolder = fileSystem.temporaryFolder
        self.testFolder = try! tempFolder.createSubfolderIfNeeded(
            withName: "CommandLineToolTests"
        )
        
        // Empty the test folder to ensure a clean state
        try! testFolder.empty()
        
        // Make the temp folder the current working folder
        let fileManager = FileManager.default
        fileManager.changeCurrentDirectoryPath(testFolder.path)
    }
    
    func testCreatingFile() throws {
        // Create an instance of the command line tool
        let arguments = [testFolder.path, "Hello.swift"]
        let tool = Opusab(arguments: arguments)
        
        // Run the tool and assert that the file was created
        try tool.run()
        XCTAssertNotNil(try? testFolder.file(named: "Hello.swift"))
    }
    
    func testNoArguments() throws {
        XCTAssertThrowsError(
            try Opusab(arguments: [testFolder.path]).run(), "") { error in
                guard let error = error as? Opusab.Error else { XCTFail(); return }
                XCTAssert(error == Opusab.Error.missingFileName)
            }
        
        
        
    }
}

