import Foundation

final class ProgressReporter {
    let string: String
    let steps: Int
    var currentStep = 1
    let verbose: Bool
    
    private var descriptionLength = 0
    private var terminated = false
    
    init(description: String, steps: Int, verbose: Bool = false) {
        self.string = description
        self.steps = steps
        self.verbose = verbose
    }
    
    func nextStep(description: String) {
        assert(currentStep <= steps)
        let line = "\(string) (\(currentStep)/\(steps)): \(description)"
        
        if verbose == false {
            print("\r" + String(repeating: " ", count: descriptionLength) + "\r", terminator: "", to: &stderr)
            print(line, terminator: "", to: &stderr)
            stderr.flush()
        } else {
            print(line, to: &stderr)
        }
        
        descriptionLength = line.count
        currentStep += 1
    }
    
    func terminate() {
        assert(terminated == false)
        print("", to: &stderr)
        terminated = true
    }
}

