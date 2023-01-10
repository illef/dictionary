{%- import "word-detail.md" as s -%}
{% for word in words %}{% call s::word_detail(word, loop.index) %}
{% endfor %}
