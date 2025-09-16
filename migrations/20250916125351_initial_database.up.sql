CREATE TABLE IF NOT EXISTS choice_questions
(
    id          SERIAL      PRIMARY KEY,
    question    TEXT        NOT NULL,
    answer1     TEXT        NOT NULL,
    answer2     TEXT        NOT NULL,
    answer3     TEXT        NOT NULL,
    answer4     TEXT        NOT NULL,
    good        SMALLINT    NOT NULL,
    theme       TEXT        NOT NULL
);

CREATE TABLE IF NOT EXISTS tip_questions
(
    id          SERIAL      PRIMARY KEY,
    question    TEXT        NOT NULL,
    good        INTEGER     NOT NULL,
    theme       TEXT        NOT NULL
);
