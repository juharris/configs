# For a nice PS1.

# `autoload` cannot be used in bash.
# It was needed when using `%{$fg[cyan]%}`, but seems like it's not needed with `%F{cyan}`.
# autoload -U colors && colors

# Case insensitive completion
autoload -Uz compinit && compinit
zstyle ':completion:*' matcher-list 'm:{a-zA-Z}={A-Za-z}'

# Press arrow keys up and down to search history by prefix.
bindkey "^[[A" history-beginning-search-backward
bindkey "^[[B" history-beginning-search-forward

# Use Home and End to go to the beginning and end of the line.
bindkey "^[[H" beginning-of-line
bindkey "^[[F" end-of-line

export CLICOLOR=1
export LSCOLORS='ExGxBxDxCxEgEdxbxgxcxd'

_current_dir=`dirname "$(realpath $0)"`
BASH_SOURCE="${_current_dir}/common.sh" emulate ksh -c '. "$BASH_SOURCE"'

autoload -Uz add-zsh-hook

typeset -g _git_branch_prompt_branch=""
typeset -g _git_branch_prompt_branch_fd=""
typeset -g _git_branch_prompt_modifiers=""
typeset -g _git_branch_prompt_modifiers_fd=""
typeset -g _git_branch_prompt_modifiers_ready=""
typeset -g _git_branch_prompt_request_id=0
typeset -g _git_branch_prompt_segment=""
typeset -g _git_branch_prompt_segment_pwd=""

# PS1 is expanded before zle starts reading input.
# The fd handler asks zle to re-expand it when the async worker returns.
function _git_branch_prompt_close_async_worker {
	emulate -L zsh
	local fd="$1"

	if [[ -n "$fd" ]]; then
		zle -F "$fd" 2> /dev/null
		{ exec {fd}<&- } 2> /dev/null
	fi
}

function _git_branch_prompt_close_async_workers {
	emulate -L zsh

	_git_branch_prompt_close_async_worker "$_git_branch_prompt_branch_fd"
	_git_branch_prompt_close_async_worker "$_git_branch_prompt_modifiers_fd"
	_git_branch_prompt_branch_fd=""
	_git_branch_prompt_modifiers_fd=""
}

function _git_branch_prompt_compute {
	emulate -L zsh
	local request_id="$1"
	local step="$2"
	local value=""

	case "$step" in
		branch)
			value="$(git_current_branch)"
			;;
		modifiers)
			value="$(git_change_modifiers)"
			;;
	esac

	print -r -- "$request_id"
	print -r -- "$step"
	print -r -- "$PWD"
	print -r -- "$value"
}

function _git_branch_prompt_precmd {
	emulate -L zsh

	_git_branch_prompt_close_async_workers
	_git_branch_prompt_branch=""
	_git_branch_prompt_modifiers=""
	_git_branch_prompt_modifiers_ready=""
	_git_branch_prompt_segment=""
	_git_branch_prompt_segment_pwd="$PWD"

	(( _git_branch_prompt_request_id += 1 ))
	_git_branch_prompt_start_async_worker branch
	_git_branch_prompt_start_async_worker modifiers
}

function _git_branch_prompt_preexec {
	emulate -L zsh

	_git_branch_prompt_close_async_workers
}

function _git_branch_prompt_render_segment {
	emulate -L zsh
	local branch="${_git_branch_prompt_branch//\%/%%}"
	local modifiers

	if [[ -n "$branch" ]]; then
		if [[ "$_git_branch_prompt_modifiers_ready" == "true" ]]; then
			modifiers="$_git_branch_prompt_modifiers"
		else
			modifiers="?"
		fi

		_git_branch_prompt_segment=" (%F{cyan}%U$branch%u%f${modifiers})"
	else
		_git_branch_prompt_segment=""
	fi

	_git_branch_prompt_segment_pwd="$PWD"
}

function _git_branch_prompt_start_async_worker {
	emulate -L zsh
	local fd step="$1"

	exec {fd}< <(_git_branch_prompt_compute "$_git_branch_prompt_request_id" "$step")
	case "$step" in
		branch)
			_git_branch_prompt_branch_fd="$fd"
			;;
		modifiers)
			_git_branch_prompt_modifiers_fd="$fd"
			;;
	esac

	if ! zle -F "$fd" _git_branch_prompt_update 2> /dev/null; then
		_git_branch_prompt_close_async_worker "$fd"
		case "$step" in
			branch)
				_git_branch_prompt_branch_fd=""
				;;
			modifiers)
				_git_branch_prompt_modifiers_fd=""
				;;
		esac
	fi
}

function _git_branch_prompt_update {
	emulate -L zsh
	local condition="$2"
	local fd="$1"
	local request_id step value worker_pwd

	if [[ -z "$condition" ]]; then
		IFS= read -r request_id <&$fd
		IFS= read -r step <&$fd
		IFS= read -r worker_pwd <&$fd
		IFS= read -r value <&$fd
	fi

	_git_branch_prompt_close_async_worker "$fd"

	if [[ "$fd" == "$_git_branch_prompt_branch_fd" ]]; then
		_git_branch_prompt_branch_fd=""
	fi

	if [[ "$fd" == "$_git_branch_prompt_modifiers_fd" ]]; then
		_git_branch_prompt_modifiers_fd=""
	fi

	if [[ -n "$condition" ]] ||
		[[ "$request_id" != "$_git_branch_prompt_request_id" ]] ||
		[[ "$worker_pwd" != "$PWD" ]]; then
		return
	fi

	case "$step" in
		branch)
			_git_branch_prompt_branch="$value"
			_git_branch_prompt_render_segment
			zle reset-prompt 2> /dev/null
			;;
		modifiers)
			_git_branch_prompt_modifiers="$value"
			_git_branch_prompt_modifiers_ready="true"
			_git_branch_prompt_render_segment
			zle reset-prompt 2> /dev/null
			;;
	esac
}

add-zsh-hook precmd _git_branch_prompt_precmd
add-zsh-hook preexec _git_branch_prompt_preexec

setopt PROMPT_SUBST;
# `%#` at the end of the line with show `#` for root, `%` for user.
# Used to use `%m` for the ${HOST}, but it would change the HOST which is annoying when other programs change it.
_host="$(hostname -s)"
PS1="%(?..%F{red}[%?]%f )%F{magenta}%n%f@%F{green}${_host}%f %F{yellow}%B%~%b%f\${_git_branch_prompt_segment} "$'\n'"%# "
