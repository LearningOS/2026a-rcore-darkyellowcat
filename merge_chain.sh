#!/bin/bash
set -euo pipefail

STATE_FILE=".merge_chain_state"

usage() {
    echo "Usage:"
    echo "  $0 start <start_branch_id> <end_branch_id>   # Start merging"
    echo "  $0 continue                                 # Continue interrupted merge"
    echo "  $0 abort                                    # Abort interrupted merge"
    echo "  $0 status                                   # Check current merge status"
    echo "  $0 sync <start_branch_id> <end_branch_id>    # Sync branches from remote"
    echo "  $0 push <remote> <start_branch_id> <end_branch_id>  # Push local branches to remote"
    echo "  $0 help                                     # Show detailed help information"
    echo ""
    echo "Examples:"
    echo "  $0 sync 1 8       # Sync ch1 to ch8 from remote (for new clones)"
    echo "  $0 start 3 8      # Start merging from ch3 to ch8"
    echo "  $0 push origin 3 8     # Push ch3 to ch8 to origin remote"
    echo "  $0 push upstream 3 8   # Push ch3 to ch8 to upstream remote"
    echo "  $0 status         # Check current merge progress"
    echo "  $0 continue       # Continue merge after resolving conflicts"
    echo ""
    echo "Merge message format: 'Merge ch{source} to ch{target}: {original message}'"
    exit 1
}

show_help() {
    echo "Git Branch Chain Merge Tool"
    echo "==========================="
    echo ""
    echo "This tool maintains linear commit relationships across multiple branches."
    echo "When you commit to branch k, it automatically merges that commit to all"
    echo "branches with numbers greater than k."
    echo ""
    echo "Command descriptions:"
    echo "  start <start_id> <end_id>  Start chain merge from ch<start_id> to ch<end_id>"
    echo "  continue                   Continue interrupted merge process"
    echo "  abort                      Abort current merge process"
    echo "  status                     Show current merge status and progress"
    echo "  sync <start_id> <end_id>   Sync local branches from remote (for new clones)"
    echo "  push <remote> <start_id> <end_id>  Push local branches to specified remote"
    echo "  help                       Show this help information"
    echo ""
    echo "Git Operations done by the tool:"
    echo "  For each target branch (start+1, start+2, ..., end):"
    echo "  1. git checkout ch{target}                # Switch to target branch"
    echo "  2. git merge ch{source} -m \"Merge...\"    # Merge with custom message"
    echo "  3. If conflict: save state and exit for manual resolution"
    echo "  4. If success: continue to next branch"
    echo ""
    echo "Workflow:"
    echo "  For newly cloned repositories:"
    echo "  0. Run '$0 sync 1 8' to create local branches from remote"
    echo ""
    echo "  Normal workflow:"
    echo "  1. Commit code to a branch (e.g., ch3)"
    echo "  2. Run '$0 start 3 8' to start merging"
    echo "  3. If conflicts occur, resolve manually and run '$0 continue'"
    echo "  4. Repeat step 3 until all branches are merged"
    echo "  5. Run '$0 push origin 3 8' to push all merged branches to remote"
    echo ""
    echo "Conflict Resolution (when merge fails):"
    echo "  1. Edit conflicted files manually"
    echo "  2. Stage resolved files with 'git add'"
    echo "  3. Complete the merge commit with 'git merge --continue'"
    echo "  4. Execute '$0 continue' to resume chain merge process"
    echo ""
    echo "Merge message format:"
    echo "  'Merge ch{source} to ch{target}: {original commit message}'"
    echo ""
    echo "  Where:"
    echo "    - source: Previous branch number (e.g., ch3)"
    echo "    - target: Current branch number (e.g., ch4)" 
    echo "    - original commit message: The commit message from the starting branch"
    echo ""
    echo "  Example: 'Merge ch3 to ch4: Add user authentication feature'"
    echo ""
    echo "State file:"
    echo "  Creates '.merge_chain_state' file to save progress during merge"
    echo "  Automatically deleted when merge completes or is aborted"
    echo ""
    exit 0
}

if [[ $# -lt 1 ]]; then
    usage
fi

COMMAND=$1

case "$COMMAND" in
    start)
        if [[ $# -ne 3 ]]; then usage; fi
        START_BRANCH=$2
        END_BRANCH=$3
        
        # Validate branch numbers
        if [[ ! $START_BRANCH =~ ^[0-9]+$ ]] || [[ ! $END_BRANCH =~ ^[0-9]+$ ]]; then
            echo "Error: Branch numbers must be numeric"
            exit 1
        fi
        
        if [[ $START_BRANCH -ge $END_BRANCH ]]; then
            echo "Error: Start branch number must be less than end branch number"
            exit 1
        fi
        
        
        # Check if all required branches exist locally
        echo "Checking required branches..."
        for ((branch_num=START_BRANCH; branch_num<=END_BRANCH; branch_num++)); do
            branch_name="ch${branch_num}"
            
            # Check if local branch exists
            if ! git show-ref --verify --quiet refs/heads/${branch_name}; then
                echo ""
                echo "❌ Error: Local branch ${branch_name} does not exist"
                echo "💡 Solution: Run the following command to sync branches from remote:"
                echo "   $0 sync ${START_BRANCH} ${END_BRANCH}"
                echo ""
                echo "This will create local branches from the remote repository."
                exit 1
            else
                echo "Local branch ${branch_name} exists"
            fi
        done
        
        # Check for incomplete merge
        if [[ -f "$STATE_FILE" ]]; then
            echo "Error: Incomplete merge process exists, please run '$0 continue' or '$0 abort' first"
            exit 1
        fi
        
        NEXT_BRANCH=$((START_BRANCH + 1))
        # Get the latest commit message from the first branch
        ORIGIN_MSG=$(git log -1 --pretty=%B ch${START_BRANCH})
        echo "Starting chain merge: ch${START_BRANCH} -> ch${END_BRANCH}"
        echo "Original commit message: $ORIGIN_MSG"
        ;;

    continue)
        if [[ ! -f "$STATE_FILE" ]]; then
            echo "No interrupted state found, cannot continue."
            exit 1
        fi
        { read -r START_BRANCH; read -r END_BRANCH; read -r NEXT_BRANCH; read -r ORIGIN_MSG; } < "$STATE_FILE"
        echo "Continuing interrupted merge from ch$NEXT_BRANCH..."
        ;;

    abort)
        if [[ -f "$STATE_FILE" ]]; then
            rm "$STATE_FILE"
            echo "Interrupted merge has been aborted."
        else
            echo "No interrupted state found, nothing to abort."
        fi
        exit 0
        ;;

    status)
        if [[ -f "$STATE_FILE" ]]; then
            { read -r START_BRANCH; read -r END_BRANCH; read -r NEXT_BRANCH; read -r ORIGIN_MSG; } < "$STATE_FILE"
            echo "Merge status: In progress"
            echo "Start branch: ch${START_BRANCH}"
            echo "End branch: ch${END_BRANCH}"
            echo "Original message: $ORIGIN_MSG"
            echo "Current progress: Preparing to merge to ch${NEXT_BRANCH}"
            echo "Remaining branches: $((END_BRANCH - NEXT_BRANCH + 1))"
            
            # Show completed branches
            if [[ $NEXT_BRANCH -gt $((START_BRANCH + 1)) ]]; then
                echo "Completed: ch${START_BRANCH} -> ch$((NEXT_BRANCH - 1))"
            fi
            
            # Show pending branches
            if [[ $NEXT_BRANCH -le $END_BRANCH ]]; then
                echo "Pending: ch${NEXT_BRANCH} -> ch${END_BRANCH}"
            fi
        else
            echo "Merge status: No merge in progress"
            echo "Current branch: $(git branch --show-current)"
        fi
        exit 0
        ;;

    sync)
        if [[ $# -ne 3 ]]; then usage; fi
        SYNC_START=$2
        SYNC_END=$3
        
        # Validate branch numbers
        if [[ ! $SYNC_START =~ ^[0-9]+$ ]] || [[ ! $SYNC_END =~ ^[0-9]+$ ]]; then
            echo "Error: Branch numbers must be numeric"
            exit 1
        fi
        
        if [[ $SYNC_START -gt $SYNC_END ]]; then
            echo "Error: Start branch number must be less than or equal to end branch number"
            exit 1
        fi
        
        # Fetch remote branches
        echo "Fetching remote branch information..."
        if ! git fetch origin --quiet; then
            echo "Error: Failed to fetch from remote repository"
            exit 1
        fi
        
        # Sync branches
        echo "Syncing branches ch${SYNC_START} to ch${SYNC_END}..."
        for ((branch_num=SYNC_START; branch_num<=SYNC_END; branch_num++)); do
            branch_name="ch${branch_num}"
            
            if ! git show-ref --verify --quiet refs/heads/${branch_name}; then
                if git show-ref --verify --quiet refs/remotes/origin/${branch_name}; then
                    echo "Creating local branch ${branch_name} from origin/${branch_name}..."
                    if ! git checkout -b ${branch_name} origin/${branch_name}; then
                        echo "Error: Failed to create local branch ${branch_name}"
                        exit 1
                    fi
                else
                    echo "Warning: Remote branch origin/${branch_name} does not exist, skipping"
                fi
            else
                echo "Local branch ${branch_name} already exists"
            fi
        done
        
        echo "Branch synchronization completed!"
        exit 0
        ;;

    push)
        if [[ $# -ne 4 ]]; then usage; fi
        REMOTE=$2
        PUSH_START=$3
        PUSH_END=$4
        
        # Validate branch numbers
        if [[ ! $PUSH_START =~ ^[0-9]+$ ]] || [[ ! $PUSH_END =~ ^[0-9]+$ ]]; then
            echo "Error: Branch numbers must be numeric"
            exit 1
        fi
        
        if [[ $PUSH_START -gt $PUSH_END ]]; then
            echo "Error: Start branch number must be less than or equal to end branch number"
            exit 1
        fi
        
        # Check for incomplete merge
        if [[ -f "$STATE_FILE" ]]; then
            echo "Warning: There is an incomplete merge process."
            echo "Please run '$0 continue' or '$0 abort' before pushing."
            exit 1
        fi
        
        # Push branches
        echo "Pushing branches ch${PUSH_START} to ch${PUSH_END} to remote '${REMOTE}'..."
        FAILED_PUSHES=()
        
        for ((branch_num=PUSH_START; branch_num<=PUSH_END; branch_num++)); do
            branch_name="ch${branch_num}"
            
            # Check if local branch exists
            if ! git show-ref --verify --quiet refs/heads/${branch_name}; then
                echo "❌ Warning: Local branch ${branch_name} does not exist, skipping"
                FAILED_PUSHES+=("${branch_name} (not found)")
                continue
            fi
            
            echo "Pushing ${branch_name}..."
            if git push ${REMOTE} ${branch_name}; then
                echo "✅ Successfully pushed ${branch_name}"
            else
                echo "❌ Failed to push ${branch_name}"
                FAILED_PUSHES+=("${branch_name} (push failed)")
            fi
        done
        
        echo ""
        if [[ ${#FAILED_PUSHES[@]} -eq 0 ]]; then
            echo "🎉 All branches pushed successfully!"
        else
            echo "⚠️ Push completed with some failures:"
            for failure in "${FAILED_PUSHES[@]}"; do
                echo "  - ${failure}"
            done
            echo ""
            echo "Please check the failed branches and try again if needed."
        fi
        
        exit 0
        ;;

    help)
        show_help
        ;;

    *)
        usage
        ;;
esac

for ((i=NEXT_BRANCH; i<=END_BRANCH; i++)); do
    CUR_BRANCH="ch${i}"
    PREV_BRANCH="ch$((i-1))"
    PROGRESS="[$((i-START_BRANCH))/$((END_BRANCH-START_BRANCH))]"

    echo ""
    echo "=== ${PROGRESS} Merging ${PREV_BRANCH} -> ${CUR_BRANCH} ==="
    
    # Check if target branch exists
    if ! git show-ref --verify --quiet refs/heads/${CUR_BRANCH}; then
        echo "Error: Branch ${CUR_BRANCH} does not exist"
        printf '%s\n' "$START_BRANCH" "$END_BRANCH" "$i" "$ORIGIN_MSG" > "$STATE_FILE"
        exit 1
    fi
    
    echo "Switching to branch ${CUR_BRANCH}..."
    if ! git checkout "${CUR_BRANCH}"; then
        echo "Error: Cannot switch to branch ${CUR_BRANCH}"
        printf '%s\n' "$START_BRANCH" "$END_BRANCH" "$i" "$ORIGIN_MSG" > "$STATE_FILE"
        exit 1
    fi

    echo "Merging ${PREV_BRANCH}..."
    if ! git merge "${PREV_BRANCH}" -m "Merge ${PREV_BRANCH} to ${CUR_BRANCH}: ${ORIGIN_MSG}"; then
        echo ""
        echo "❌ Conflict occurred in ${CUR_BRANCH}"
        echo "Please follow these steps to resolve:"
        echo "1. Manually resolve conflict files"
        echo "2. Run 'git add .' to add resolved files"
        echo "3. Run 'git commit' to commit the merge"
        echo "4. Run '$0 continue' to continue the merge process"
        echo ""
        echo "Or run '$0 abort' to abort the merge"
        echo "Or run '$0 status' to check current status"
        printf '%s\n' "$START_BRANCH" "$END_BRANCH" "$i" "$ORIGIN_MSG" > "$STATE_FILE"
        exit 1
    fi
    
    echo "✅ Successfully merged to ${CUR_BRANCH}"
done

# Merge completed, delete state file
[[ -f "$STATE_FILE" ]] && rm "$STATE_FILE"
echo ""
echo "🎉 All branches merged successfully!"
echo "Merge range: ch${START_BRANCH} -> ch${END_BRANCH}"
echo "Original commit: $ORIGIN_MSG"
echo ""
echo "💡 To push all merged branches to remote, run:"
echo "   $0 push origin ${START_BRANCH} ${END_BRANCH}"
