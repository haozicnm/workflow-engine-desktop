$task = Get-Content 'C:\Users\haozi\Dev\workflow-engine-desktop\.reasonix\task-v8.2.md' -Raw
Write-Output "Task length: $($task.Length)"
reasonix run --max-steps 80 $task 2>&1
