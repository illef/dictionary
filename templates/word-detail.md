{% macro word_detail(word, index) %}
## case {{ index }}
{% for meaning in word.meanings %}
**{{ meaning.part_of_speech }}**

{% for definition in meaning.definitions %}- {{ definition.definition }}
{% endfor %}{% endfor %}{% endmacro %}
