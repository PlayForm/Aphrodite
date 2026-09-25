# Aphrodite 💋 v{{ version }}

{% for group, commits in commits | group_by(attribute="group") %}
### {{ group }}

{% for commit in commits %}
- {{ commit.message | upper_first }}\
  {% if commit.github.pr_number %} ([#{{ commit.github.pr_number }}](https://github.com/PlayForm/Aphrodite/issues/{{ commit.github.pr_number }})){% endif %}
{% endfor %}
{% endfor %}

**Full Changelog**: https://github.com/PlayForm/Aphrodite/compare/{{ previous_tag }}...Aphrodite/v{{ version }}