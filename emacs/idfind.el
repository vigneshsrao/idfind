
;;; idfind.el --- emacs interface for `idfind'

;;; To install, merely put this file somewhere GNU Emacs will find it,
;;; then add the following lines to your .emacs file:
;;;
;;;   (autoload 'idfind "idfind" nil t)
;;;
;;; You may also adjust some customizations variables, below, by defining
;;; them in your .emacs file.

(require 'compile)
(require 'thingatpt)

(defvar idfind-command "idfind " "The command run by the idfind function.")

(defvar idfind-mode-font-lock-keywords
  '(("^\\(Compilation\\|idfind\\) \\(started\\|finished\\).*"
     (0 '(face nil message nil help-echo nil mouse-face nil) t))))

(defvar use-idfind-in-buffer-name t
  "If non-nil, use the search string in the idfind buffer's name.")

(define-compilation-mode idfind-mode "IDFind"
  "Specialization of compilation-mode for use with idfind."
  nil)

;;;###autoload
(defun idfind (args)
  "Run idfind, with user-specified ARGS, and collect output in a buffer.
While idfind runs asynchronously, you can use the \\[next-error] command to
find the text that idfind hits refer to. The command actually run is
defined by the idfind-command variable."
  (interactive (list (read-shell-command
     (concat "Run " idfind-command " (with args): ") (thing-at-point 'symbol))))
  (let (compile-command
	(compilation-error-regexp-alist grep-regexp-alist)
	(compilation-directory default-directory)
	(idfind-full-buffer-name (concat "*idfind-buf*")))
    (save-some-buffers (not compilation-ask-about-save) nil)
    (compilation-start (concat idfind-command "-m search -d " (projectile-project-root) "sdb.json" " -e \"" args "\"") 'idfind-mode
		         (function (lambda (ignore)
		        	     idfind-full-buffer-name))
		       (regexp-quote args))))

(provide 'dbidfind)
