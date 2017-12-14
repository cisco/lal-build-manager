# lal(1) completion

_lal()
{
    local cur prev words cword
    _init_completion || return

    local -r subcommands="build clean configure export fetch help init script run ls
                          query remove rm shell stash save status update upgrade verify
                          publish env list-components list-supported-environments list-dependencies
                          list-environments list-configurations propagate"

    local has_sub
    for (( i=0; i < ${#words[@]}-1; i++ )); do
        if [[ ${words[i]} == @(build|clean|configure|export|script|propagate|fetch|help|init|remove|rm|script|run|query|shell|stash|save|status|ls|update|upgrade|verify|publish|env) ]]; then
            has_sub=1
        fi
    done

    local in_lal_repo=""
    if [ -f "$PWD/.lal/manifest.json" ] || [ -f "$PWD/manifest.json" ]; then
        in_lal_repo="1"
    fi

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
        if [[ ${words[i]} == @(build|remove|rm|propagate|export|init|update|script|run|status|ls|query|shell|publish|env|configure|help) ]]; then
            special=${words[i]}
        fi
    done

    if [[ -n $special ]]; then
        case $special in
            build)
                # lal can get the keys from manifest.components
                [[ $in_lal_repo ]] || return 0
                local -r components=$(lal list-components)
                if [[ $prev = "build" ]]; then
                    COMPREPLY=($(compgen -W "$components" -- "$cur"))
                elif [[ $prev == @(--config|-c) ]]; then
                    # Identify which component is used (arg after build that's not a flag)
                    local build_component i
                    for (( i=2; i < ${#words[@]}-1; i++ )); do
                        if [[ ${words[i]} != -* ]]; then
                            build_component=${words[i]}
                        fi
                    done
                    local -r configs="$(lal list-configurations "${build_component}")"
                    COMPREPLY=($(compgen -W "$configs" -- "$cur"))
                else
                    # suggest flags
                    local -r build_flags="-r --release -f --force -c --config -h --help --X11 -X -n --net-host --print-only --simple-verify -s --env-var"
                    COMPREPLY=($(compgen -W "$build_flags" -- "$cur"))
                fi
                ;;
            publish)
                # lal can get the keys from manifest.components
                [[ $in_lal_repo ]] || return 0
                local -r components=$(lal list-components)
                if [[ $prev = "publish" ]]; then
                    COMPREPLY=($(compgen -W "$components" -- "$cur"))
                fi
                ;;
            env)
                [[ $in_lal_repo ]] || return 0
                local -r env_subs="set reset update help -h --help"
                if [[ $prev = "set" ]]; then
                    local -r envs="$(lal list-supported-environments)"
                    COMPREPLY=($(compgen -W "$envs" -- "$cur"))
                else
                    COMPREPLY=($(compgen -W "$env_subs" -- "$cur"))
                fi
                ;;
            init)
                if [[ $prev = "init" ]]; then
                    local -r envs="$(lal list-environments)"
                    COMPREPLY=($(compgen -W "$envs" -- "$cur"))
                fi
                ;;
            status|ls)
                [[ $in_lal_repo ]] || return 0
                local -r ls_flags="-f --full -o --origin -t --time -h --help"
                COMPREPLY=($(compgen -W "$ls_flags" -- "$cur"))
                ;;
            export|query)
                components=$(find "$HOME/.lal/cache/environments" -maxdepth 2 -mindepth 2 -type d -printf "%f " 2> /dev/null)
                COMPREPLY=($(compgen -W "$components" -- "$cur"))
                ;;
            update)
                [[ $in_lal_repo ]] || return 0
                # Looking in local cache for allowed component names
                # Means this won't work first time, but will be quick
                local components=""
                components=$(find "$HOME/.lal/cache/environments/" -maxdepth 2 -mindepth 2 -type d -printf "%f " 2> /dev/null)
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
                [[ $in_lal_repo ]] || return 0
                # look in INPUT here, nothing else makes sense
                local -r installed=$(find "$PWD/INPUT/" -maxdepth 1 -mindepth 1 -type d -printf "%f " 2> /dev/null)
                COMPREPLY=($(compgen -W "$installed" -- "$cur"))
                ;;
            propagate)
                [[ $in_lal_repo ]] || return 0
                # look in INPUT here, nothing else makes sense
                local -r installed=$(find "$PWD/INPUT/" -maxdepth 1 -mindepth 1 -type d -printf "%f " 2> /dev/null)
                COMPREPLY=($(compgen -W "$installed" -- "$cur"))
                ;;
            shell)
                [[ $in_lal_repo ]] || return 0
                # suggest flags
                local -r sh_flags="-p --privileged -h --help --print-only --X11 -X -n --net-host --env-var"
                if [[ $prev = "shell" ]]; then
                    COMPREPLY=($(compgen -W "$sh_flags" -- "$cur"))
                fi
                ;;
            configure)
                # figure out what type of lal installation we have
                # and from that infer where the configs would be
                local -r run_pth=$(readlink -f "$(which lal)")
                local config_dir;
                if [[ $run_pth == *target/debug/lal ]] || [[ $run_pth == *target/release/lal ]]; then
                    # compiled lal => configs in the source dir (up from the target build dir)
                    config_dir="${run_pth%/target/*}/configs"
                else
                    # musl release => configs in prefix/share/lal/configs
                    config_dir="${run_pth%/bin/*}/share/lal/configs"
                fi
                local -r configs=$(find "$config_dir" -type f)
                COMPREPLY=($(compgen -W "$configs" -- "$cur"))
                ;;
            help)
                COMPREPLY=($(compgen -W "$subcommands" -- "$cur"))
                ;;
            script|run)
                [[ $in_lal_repo ]] || return 0
                # locate the scripts in .lal/scripts
                local -r scripts="$(find "$PWD/.lal/scripts/" -maxdepth 1 -type f -printf "%f " 2> /dev/null)"
                local -r second_args="${scripts} -p --privileged --X11 -X -n --net-host --print-only --env-var"

                if [[ $prev == @(script|run) ]] || [[ $prev == -* ]]; then
                    COMPREPLY=($(compgen -W "$second_args" -- "$cur"))
                else
                    # Identify which script we used (arg after run that's not a flag)
                    local run_script i
                    for (( i=2; i < ${#words[@]}-1; i++ )); do
                        if [[ ${words[i]} != -* ]] && echo "$scripts" | grep -q "${words[i]}"; then
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
