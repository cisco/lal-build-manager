# lal(1) completion

_lal()
{
    local cur prev words cword
    _init_completion || return

    local -r subcommands="build clean configure export fetch help init script run ls
                          query remove shell stash save status update upgrade verify
                          publish env list-components list-dependencies list-environments"

    local has_sub
    for (( i=0; i < ${#words[@]}-1; i++ )); do
        if [[ ${words[i]} == @(build|clean|configure|export|script|fetch|help|init|remove|rm|script|run|query|shell|stash|save|status|ls|update|upgrade|verify|publish|env) ]]; then
            has_sub=1
        fi
    done

    # global flags
    if [[ $prev = 'lal' && "$cur" == -* ]]; then
        COMPREPLY=( $(compgen -W '-v -h -V --version --help' -- "$cur" ) )
        return 0
    fi
    # first subcommand
    if [[ -z "$has_sub" ]]; then
        COMPREPLY=( $(compgen -W "$subcommands" -- "$cur" ) )
        return 0
    fi

    # special subcommand completions
    local special i
    for (( i=0; i < ${#words[@]}-1; i++ )); do
        if [[ ${words[i]} == @(build|remove|rm|export|update|script|run|status|ls|query|shell|publish|env) ]]; then
            special=${words[i]}
        fi
    done

    if [[ -n $special ]]; then
        case $special in
            build)
                # lal can get the keys from manifest.components
                [ -f "$PWD/manifest.json" ] || return 0
                local -r components=$(lal list-components)
                if [[ $prev = "build" ]]; then
                    COMPREPLY=($(compgen -W "$components" -- "$cur"))
                else
                    # suggest flags
                    local -r build_flags="-r --release -f --force -c --config -h --help --print-only"
                    COMPREPLY=($(compgen -W "$build_flags" -- "$cur"))
                fi
                ;;
            publish)
                # lal can get the keys from manifest.components
                [ -f "$PWD/manifest.json" ] || return 0
                local -r components=$(lal list-components)
                if [[ $prev = "publish" ]]; then
                    COMPREPLY=($(compgen -W "$components" -- "$cur"))
                fi
                ;;
            env)
                [ -f "$PWD/manifest.json" ] || return 0
                local -r env_subs="set reset update help -h --help"
                if [[ $prev = "set" ]]; then
                    local -r envs="$(lal list-environments)"
                    COMPREPLY=($(compgen -W "$envs" -- "$cur"))
                else
                    COMPREPLY=($(compgen -W "$env_subs" -- "$cur"))
                fi
                ;;
            status|ls)
                [ -f "$PWD/manifest.json" ] || return 0
                local -r ls_flags="-f --full -o --origin -t --time -h --help"
                COMPREPLY=($(compgen -W "$ls_flags" -- "$cur"))
                ;;
            export|query)
                components=$(find "$HOME/.lal/cache/globals/" -maxdepth 1 -mindepth 1 -type d -printf "%f " 2> /dev/null)
                COMPREPLY=($(compgen -W "$components" -- "$cur"))
                ;;
            update)
                [ -f "$PWD/manifest.json" ] || return 0
                # Looking in local cache for allowed component names
                # Means this won't work first time, but will be quick
                local components=""
                components=$(find "$HOME/.lal/cache/globals/" -maxdepth 1 -mindepth 1 -type d -printf "%f " 2> /dev/null)
                # also add stashed components to list
                for dr in ~/.lal/cache/stash/**/**; do
                    if [[ "$dr" != *"**" ]]; then # ignore empty element (ends in **)
                        components="${components} $(basename "$(dirname "$dr")")=$(basename "$dr")"
                    fi
                done
                # can't complete past the equals because = is a new word for some reason
                # but at least you have the info in the list - #bash
                COMPREPLY=($(compgen -W "$components" -- "$cur"))
                ;;
            remove|rm)
                [ -f "$PWD/manifest.json" ] || return 0
                # look in INPUT here, nothing else makes sense
                local -r installed=$(find "$PWD/INPUT/" -maxdepth 1 -mindepth 1 -type d -printf "%f " 2> /dev/null)
                COMPREPLY=($(compgen -W "$installed" -- "$cur"))
                ;;
            shell)
                [ -f "$PWD/manifest.json" ] || return 0
                # suggest flags
                local -r sh_flags="-p --privileged -h --help --print-only"
                if [[ $prev = "shell" ]]; then
                    COMPREPLY=($(compgen -W "$sh_flags" -- "$cur"))
                fi
                ;;
            script|run)
                [ -f "$PWD/manifest.json" ] || return 0
                # locate the scripts in .lal/scripts
                local -r scripts=$(find "$PWD/.lal/scripts/" -type f -printf "%f " 2> /dev/null)
                if [[ $prev == @(script|run) ]]; then
                    COMPREPLY=($(compgen -W "$scripts" -- "$cur"))
                else
                    # Identify which script we used (arg after run)
                    local run_script i
                    for (( i=2; i < ${#words[@]}-1; i++ )); do
                        if echo "$scripts" | grep -q "${words[i]}"; then
                            run_script=${words[i]}
                        fi
                    done
                    local -r comps=$(source "$PWD/.lal/scripts/$run_script"; completer)
                    COMPREPLY=($(compgen -W "$comps" -- "$cur"))
                fi
                ;;
        esac
    fi

    return 0
} &&
complete -F _lal lal

